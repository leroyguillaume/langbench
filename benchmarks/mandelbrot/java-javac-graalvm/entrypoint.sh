#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.java
# The bytecode the image ships, compiled once at `docker build`. This is what
# `run` executes.
CLASSES=${CLASSES:-/usr/local/lib/mandelbrot}
MAIN_CLASS=Mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# Graal is GraalVM's default JIT, and this says so out loud anyway.
#
# The whole point of this row is that the hot loop is compiled by Graal rather than
# by C2 -- and a row that silently fell back to C2 would be `java-javac-openjdk`
# wearing a different name, publishing a difference that is really just noise.
# HotSpot refuses to start on a flag it does not recognise, so if a future GraalVM
# drops JVMCI these flags fail the campaign instead of quietly measuring the wrong
# compiler. `-Djdk.graal.ShowConfiguration=info` prints which compiler was actually
# loaded; it goes to stderr, so run `docker run ... run 64 50 1` and read it there.
JIT_FLAGS="-XX:+EnableJVMCI -XX:+UseJVMCICompiler"

# The ISA this run actually got, reported on stdout with the numbers it explains.
#
# A constant, and swapping the code generator does not make it a variable: Graal
# compiles the hot loop while the program runs, on the machine it is running on, and
# there is no `-march` to hand a JIT. `native` is not a preference this backend
# expressed -- it is the only thing a JIT can do, and the mode says so where it used
# to be an apology.
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
# the timed rebuild. There is nothing to tune: javac emits bytecode, and every
# decision that matters to this benchmark is made later, by the JIT.
compile_to() {
    output=$1
    mkdir -p "${output}"
    javac -d "${output}" "${SOURCE}"
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
    # `threads` is accepted for contract compliance and deliberately ignored: javac
    # on a single file has nothing to parallelise.
    #
    # This measures javac, which is source -> bytecode and nothing more. The machine
    # code this program actually runs does not exist yet: Graal emits it during the
    # *run*, once the loop is hot. So the Build column here is a fact
    # about javac, not a number to rank against gcc's -- and the JIT's compile time
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
    # -- here, the JVM booting -- and it is a result rather than overhead to be
    # subtracted. It is the largest such gap in the table, and that is the point.
    #
    # No -Xss, no -Xmx, no -XX:TieredStopAtLevel, and no cap on vector width: the
    # defaults are what a Java program gets, and tuning them would measure our tuning.
    # JIT_FLAGS is two flags and must split; quoting it would hand the JVM one
    # argument with a space in it.
    # shellcheck disable=SC2086
    output=$(java ${JIT_FLAGS} -cp "${CLASSES}" "${MAIN_CLASS}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":"%s","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${ISA}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # This is the bytecode javac emitted, not the machine code Graal ends at -- which
    # lives in memory, and reading it needs an hsdis plugin the JDK does not ship.
    # Compare it with the Kotlin and Scala rows': all three feed the same JIT, so
    # their bytecode is where the differences between them are visible.
    listing=$(javap -c -p -cp "${CLASSES}" "${MAIN_CLASS}")
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
