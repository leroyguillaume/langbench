#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.kt
# The bytecode the image ships, compiled once at `docker build`. This is what
# `run` executes.
CLASSES=${CLASSES:-/usr/local/lib/mandelbrot}
KOTLIN_HOME=${KOTLIN_HOME:-/usr/local/lib/kotlinc}
# The Kotlin stdlib is the language's runtime, not a dependency -- libstdc++, not
# rayon -- and it must be on the classpath for the program to run at all.
STDLIB=${KOTLIN_HOME}/lib/kotlin-stdlib.jar
# kotlinc names the class after the file, with a Kt suffix: top-level functions
# have to live somewhere, and that somewhere is `MandelbrotKt`.
MAIN_CLASS=MandelbrotKt
BUILD_DIR=${BUILD_DIR:-/build}

# The ISA this run actually got, reported on stdout with the numbers it explains.
#
# A constant. kotlinc emits bytecode, which targets no CPU at all; the machine code
# appears only when C2 compiles the hot loop, on the machine it is running on, from
# the instruction set it finds there. There is no `-march` anywhere in this pipeline
# to hand a baseline to. `native` is not a preference this backend expressed -- it is
# the only thing a JIT can do, and the mode now says so.
ISA=native

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

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild. There is nothing to tune: kotlinc emits bytecode, and every
# decision that matters to this benchmark is made later, by the JIT.
#
# `-nowarn` only silences; it changes no codegen. kotlinc is a slow compiler and
# that is one of the things this row is here to report, so nothing here tries to
# make it look quick -- no incremental mode, no daemon, no build tool.
compile_to() {
    output=$1
    mkdir -p "${output}"
    "${KOTLIN_HOME}/bin/kotlinc" -nowarn -d "${output}" "${SOURCE}"
}

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       compile the shipped bytecode (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        disassemble the hot loop (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1

case "${phase}" in
install)
    compile_to "${CLASSES}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored: kotlinc
    # on a single file has nothing to parallelise.
    #
    # This measures kotlinc, which is source -> bytecode and nothing more. The machine
    # code this program actually runs does not exist yet: HotSpot's C2 emits it
    # during the *run*, once the loop is hot. So the Build column here is a fact
    # about kotlinc, not a number to rank against gcc's -- and the JIT's compile time
    # is billed to the run column, where it happens.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    compile_to "${BUILD_DIR}/classes" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. A .class file is
    # bytecode, and putting its size next to an ELF's would rank packaging, not
    # codegen.
    printf '{"phase":"build","elapsed_ns":%s,"isa":"%s","user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${ISA}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`.
    # The gap between this and the harness's external clock is runtime startup cost
    # -- here, the JVM booting and loading the Kotlin stdlib -- and it is a result
    # rather than overhead to be subtracted.
    #
    # No -Xss, no -Xmx, no -XX:TieredStopAtLevel, and no cap on vector width: the
    # defaults are what a Kotlin program gets, and tuning them would measure our
    # tuning.
    output=$(java -cp "${CLASSES}:${STDLIB}" "${MAIN_CLASS}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":"%s","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${ISA}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # This is the bytecode kotlinc emitted, not the machine code C2 ends at -- reading
    # *that* needs an hsdis plugin the JDK does not ship, and it lives in memory
    # anyway. Compare it with the Java and Scala rows': all three feed the same
    # JIT, so their bytecode is where the differences between them are visible.
    listing=$(javap -c -p -cp "${CLASSES}:${STDLIB}" "${MAIN_CLASS}")
    if ! printf '%s\n' "${listing}" | grep -q 'rowIterations'; then
        printf 'no bytecode for rowIterations in %s\n' "${CLASSES}" >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
