#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.scala
# An ELF, not a class file: this row has no JVM at run time.
BINARY=/usr/local/bin/mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# Scala Native has one floating-point semantics, and it is the strict one. It emits
# LLVM IR and clang compiles it -- the same code generator the C rows use, reached the
# same way -- so `-ffp-contract=off` is passed in `compile_to()` for exactly the reason
# c-clang passes it: clang contracts into an FMA by default, and the checksum is not
# negotiable. Unconditionally, in every mode: the ISA target chooses which instructions
# clang may emit and never what the arithmetic means, and `-ffast-math` is not something
# this backend has to offer, because a run that computes a different number is not a
# faster run.

now_ns() {
    date +%s%N
}

# CPU time comes from the cgroup, never from the harness's `rusage`: the workload
# runs in a different process tree from the `docker` client.
read_cpu_time() {
    user_usec=0
    system_usec=0
    if [ -r /sys/fs/cgroup/cpu.stat ]; then
        user_usec=$(awk '/^user_usec/ { print $2 }' /sys/fs/cgroup/cpu.stat)
        system_usec=$(awk '/^system_usec/ { print $2 }' /sys/fs/cgroup/cpu.stat)
    fi
}

# The peak memory the container needed, from the cgroup's own high-water mark.
# Not the RSS of one process: it is the whole container -- the process tree, the
# page cache it faulted in, the tmpfs it wrote. That is the memory this backend
# needed to run, which is the number worth publishing.
read_peak_memory() {
    peak_bytes=null
    if [ -r /sys/fs/cgroup/memory.peak ]; then
        peak_bytes=$(cat /sys/fs/cgroup/memory.peak)
    elif [ -r /sys/fs/cgroup/memory/memory.max_usage_in_bytes ]; then
        peak_bytes=$(cat /sys/fs/cgroup/memory/memory.max_usage_in_bytes)
    fi
}

# The ISA target. Unlike every JVM row, this one honours it exactly: the code generator
# is clang, and clang takes gcc's spelling verbatim -- the same `-march=x86-64-v3`,
# `-march=armv8.2-a` or `-march=native` the C rows are built with. No translation table,
# no approximation, no cap, and so nothing for `isa` to disagree with `mode` about.
march_flag() {
    if [ -n "${MARCH:-}" ]; then
        printf -- '-march=%s\n' "${MARCH}"
    fi
}

# What clang was actually targeted at: the `-march` it got, spelled the way it got it.
# A JSON value, not a string -- nothing pinned is `null`, an absence rather than a claim.
#
# `native` is reported as `native`, and clang is not asked to resolve it into a
# microarchitecture name. That query is another compiler process, and a process spawned
# inside a measured phase lands in the very cgroup the CPU column is read from: the name
# would cost a measurement.
resolve_isa() {
    if [ -n "${MARCH:-}" ]; then
        isa="\"${MARCH}\""
    else
        isa=null
    fi
}

# Single source of truth for the build, shared by the image build and by the timed
# rebuild.
#
# `--server=false`: scala-cli would otherwise start a Bloop build server and keep it
# alive between invocations. That server is a warm compiler daemon -- exactly the
# kind of caching that makes a Build column lie -- so it is off, and every build here
# is a cold one.
#
# `--workspace` points the project's own build cache at a directory the caller
# chooses. The timed build puts it in the tmpfs, so our code compiles from scratch,
# while COURSIER_CACHE stays warm with the Scala Native runtime jars. Hot for the
# toolchain, cold for our code.
#
# `--native-compile` hands flags straight to clang, which is what makes the ISA target
# real here rather than aspirational -- and what makes `-ffp-contract=off` a guarantee
# rather than a hope.
compile_to() {
    output=$1
    workspace=$2
    mkdir -p "${workspace}"

    # shellcheck disable=SC2046
    scala-cli --power package \
        --native \
        --native-mode release-fast \
        --native-compile "$(march_flag)" \
        --native-compile -ffp-contract=off \
        --scala "${SCALA_VERSION}" \
        --server=false \
        --workspace "${workspace}" \
        --force \
        -o "${output}" \
        "${SOURCE}"
}

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       compile the shipped binary (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        disassemble the hot loop (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1
# Resolved once, up front, for every phase: the measured phases have to report the ISA
# the shipped binary was built for, and that is the one the image was built with.
resolve_isa

case "${phase}" in
install)
    # Runs at `docker build`, where the network still exists: this is what pulls the
    # Scala Native runtime, the compiler plugin and the standard library into
    # COURSIER_CACHE, so the timed build can compile offline.
    compile_to "${BINARY}" /tmp/workspace
    rm -rf /tmp/workspace
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # scala-cli takes no -j, and parallelises its own linking internally.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    compile_to "${BUILD_DIR}/mandelbrot" "${BUILD_DIR}/workspace" >&2
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped binary, measured after the timer stops. We never
    # strip during the timed build: that would add link-time work to the number.
    cp "${BINARY}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${BINARY}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    # Only .text is comparable across implementations: the binary carries a garbage
    # collector and a reimplemented slice of the JDK, which the Binary column would
    # read as bloat and the .text column reads as what it is.
    text_bytes=$(size --format=sysv "${BINARY}" | awk '/^\.text/ { print $2 }')

    read_cpu_time
    read_peak_memory
    printf '{"phase":"build","elapsed_ns":%s,"isa":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${isa}" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # No JVM, no classpath: an ELF, started like one. Read this row's Compute column
    # against scala-scalac-openjdk to see what the JIT's warm-up was costing, and its
    # Startup against the JVM rows to see what booting a VM was costing.
    output=$("${BINARY}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${isa}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # Evidence, not measurement -- and real machine code, from LLVM, where the JVM
    # Scala rows can only show bytecode. Scala Native mangles its symbols, so match
    # on the readable part rather than spelling the whole thing.
    symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
        | grep -m1 'rowIterations' || true)
    if [ -z "${symbol}" ]; then
        printf 'no rowIterations symbol in %s: it was inlined, or the mangling changed\n' "${BINARY}" >&2
        exit 1
    fi
    listing=$(objdump --disassemble="${symbol}" --no-show-raw-insn "${BINARY}")
    if ! printf '%s\n' "${listing}" | grep -qE '^[[:space:]]+[0-9a-f]+:'; then
        printf 'empty listing for %s: it is not code\n' "${symbol}" >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
