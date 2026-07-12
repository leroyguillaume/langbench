#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.scala
# An ELF, not a class file: this row has no JVM at run time.
BINARY=/usr/local/bin/mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# Scala Native has one floating-point semantics, and it is the strict one. It emits
# LLVM IR and clang compiles it -- so the flags below are the *same* ones the C rows
# use, reaching the same code generator. `-ffp-contract=off` is passed for exactly
# the reason c-clang passes it: clang contracts into FMA by default, and the
# checksum is not negotiable.
#
# `fma` and `fast` are not offered, though the compiler underneath would understand
# them. Scala the language says nothing about fusing, and a fused build here would
# be a claim about clang's flags rather than about Scala Native -- so this backend
# distinguishes the one mode it can honestly speak for.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: this backend distinguishes only strict; mode %s is built as strict\n' \
            "${FP_MODE}" >&2
        ;;
    *)
        printf 'unknown FP_MODE: %s\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac
}

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

# The ISA baseline. Unlike every JVM row, this one can honour it exactly: the code
# generator is clang, and clang takes gcc's spelling verbatim -- the same
# `-march=x86-64-v3` or `-march=armv8.2-a` the C rows are built with. No
# translation table, no approximation, no cap.
march_flag() {
    if [ -n "${MARCH:-}" ]; then
        printf -- '-march=%s\n' "${MARCH}"
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
# `--native-compile` hands flags straight to clang, which is what makes the ISA
# baseline and the FP mode real here rather than aspirational.
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
check_fp_mode

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
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" \
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
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
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
