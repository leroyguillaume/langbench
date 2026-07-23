#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.kt
# kotlinc names the class after the file, with a Kt suffix.
MAIN_CLASS=MandelbrotKt
# The *binary* the image ships -- an ELF, not a class file. This backend is the one
# JVM row with a real executable, and the size columns describe it.
BINARY=/usr/local/bin/mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}
KOTLIN_HOME=${KOTLIN_HOME:-/usr/local/lib/kotlinc}
# The Kotlin stdlib is the language's runtime, and native-image must swallow it:
# it goes on the classpath for the AOT compile, and ends up *inside* the binary.
# That is the whole reason this row is not a rerun of java-native-image.
STDLIB=${KOTLIN_HOME}/lib/kotlin-stdlib.jar

# Floating point is strict here, and there is no flag that says so -- because there is
# nothing to say. Since JEP 306 (Java 17) the JLS evaluates every expression as if
# `strictfp`, and it binds the code generator whether that generator runs before the
# program or during it: Graal ahead of time may no more contract `a * b + c` into an
# FMA than C2 on the fly. So the checksum is HotSpot's, and it is C's, in every mode.
# The ISA target below widens the instructions Graal may emit; it cannot touch what the
# arithmetic means, and native-image has no fast-math to be asked for.

# The ISA target -- and this backend is the only Kotlin row that has one at all.
#
# native-image is an ahead-of-time compiler, so unlike HotSpot it takes an honest
# `-march`, `native` included -- `native` is in fact its default, and it is passed
# explicitly all the same, because a default is not a decision.
#
# THE RULE FOR THE BASELINE IS: NEVER ABOVE THE CAMPAIGN'S. On x86-64 the mapping is
# exact (`x86-64-v3` is a name native-image knows). On AArch64 it is not: native-image
# offers `armv8-a` and `armv8.1-a` and stops there, with no `armv8.2-a` to give. So the
# baseline here is the highest one it *can* express that does not exceed the one every
# other backend was held to -- one level below. It is handicapped rather than flattered,
# which is the safe direction to be wrong in.
#
# Which is precisely why a sample carries `isa` beside its `mode`: the mode says
# `baseline` and this row says `armv8.1-a`, and that disagreement is published rather
# than buried in a manifest's prose. `isa` is what was passed to the compiler, never
# what the harness asked for. It is a JSON value, not a string -- nothing pinned is
# `null`, an absence rather than a claim.
resolve_isa() {
    case "${MARCH:-}" in
    '')
        march_flag=-march=compatibility
        isa='"compatibility"'
        ;;
    native)
        march_flag=-march=native
        isa='"native"'
        ;;
    x86-64-v3)
        march_flag=-march=x86-64-v3
        isa='"x86-64-v3"'
        ;;
    armv8.2-a)
        march_flag=-march=armv8.1-a
        isa='"armv8.1-a"'
        ;;
    *)
        printf 'unknown MARCH for native-image: %s. Run "native-image -march=list" and add the\n' "${MARCH}" >&2
        printf 'highest baseline that does not EXCEED it -- never one above.\n' >&2
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

# Single source of truth for the build, shared by the image build and by the timed
# rebuild. Two compilers, one pipeline: kotlinc emits bytecode, and native-image --
# Graal, running ahead of time -- turns that bytecode into an ELF. There is no JVM
# left at run time, which is the whole point of the row.
#
# `-H:-DeleteLocalSymbols` keeps the symbol table. It changes no code -- symbols are
# not instructions -- but without it `nm` reports "no symbols", the shipped binary
# arrives pre-stripped, and both the Binary/Stripped columns and `disasm` would have
# nothing to say. Every other compiled row here ships unstripped and strips a copy
# afterwards; this keeps that comparison honest.
#
# `--no-fallback`: without it, a build that cannot close the world silently emits a
# *JVM-requiring* launcher instead of a native binary, and this row would quietly
# become java-javac-openjdk with extra steps.
compile_to() {
    output=$1
    classes=$2
    mkdir -p "${classes}"
    "${KOTLIN_HOME}/bin/kotlinc" -nowarn -d "${classes}" "${SOURCE}"
    native-image "${march_flag}" -O3 --no-fallback -H:-DeleteLocalSymbols \
        -cp "${classes}:${STDLIB}" "${MAIN_CLASS}" -o "${output}"
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
# Resolved once, up front, for every phase: the compiling phases need the flag, and the
# measured phases need `isa` to report -- the ISA the shipped binary was built for is
# the one the image was built with, and this is where both are read.
resolve_isa

case "${phase}" in
install)
    compile_to "${BINARY}" /tmp/classes
    rm -rf /tmp/classes
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # native-image parallelises its own analysis across every core it finds, and
    # takes no -j. It is the most expensive build in this table by a wide margin --
    # a whole-program points-to analysis, then Graal codegen for everything it
    # reached -- and that cost is exactly what the Build column is for. AOT is not
    # free; it is prepaid.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    compile_to "${BUILD_DIR}/mandelbrot" "${BUILD_DIR}/classes" >&2
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped binary, measured after the timer stops. We never
    # strip during the timed build: that would add work to the number.
    cp "${BINARY}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${BINARY}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    # Only .text is comparable across implementations -- and here it is large,
    # because a native-image binary carries the pieces of the JDK the program
    # reached, plus a garbage collector. That is not bloat, it is the runtime,
    # linked in. The JVM rows pay for it too; they just do not have to ship it.
    text_bytes=$(size --format=sysv "${BINARY}" | awk '/^\.text/ { print $2 }')

    read_cpu_time
    read_peak_memory
    printf '{"phase":"build","elapsed_ns":%s,"isa":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${isa}" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # No `java`, no classpath, no JVM: this is an ELF, and it starts like one.
    #
    # The result is not in the Startup column, though -- container creation dominates
    # that for every fast backend, and this row's is no lower than the JVM's. It is in
    # Compute: the binary arrives already compiled, where a JIT is still warming up
    # inside the region we are timing. On this workload that is the whole difference.
    output=$("${BINARY}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${isa}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # Evidence, not measurement -- and for this row it is real machine code, where
    # every other JVM backend can only show bytecode. Graal's AOT output, in an ELF,
    # readable by the same objdump that reads the C kernel's.
    #
    # The symbol carries a generated hash (`Mandelbrot_rowIterations_vWBxAY...`), so
    # match on the stable part rather than spelling it.
    symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
        | grep -m1 'rowIterations' || true)
    if [ -z "${symbol}" ]; then
        printf 'no rowIterations symbol in %s: was it built without -H:-DeleteLocalSymbols?\n' "${BINARY}" >&2
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
