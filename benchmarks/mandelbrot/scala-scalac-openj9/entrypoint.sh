#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/Mandelbrot.scala
# The bytecode the image ships, compiled once at `docker build`. This is what
# `run` executes.
CLASSES=${CLASSES:-/usr/local/lib/mandelbrot}
SCALA_HOME=${SCALA_HOME:-/usr/local/lib/scala3}
# The Scala runtime -- both halves of it. Scala 3 ships its own library *and* still
# stands on the 2.13 one; they are the language's runtime, not dependencies, and
# both must be on the classpath for the program to run at all. Their loading cost
# is part of what this backend costs, and it is visible in the Startup column.
STDLIB="${SCALA_HOME}/lib/*"
MAIN_CLASS=Mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# The ISA this run actually got, reported on stdout with the numbers it explains.
#
# A constant. scalac emits bytecode, which targets no CPU at all; the machine code
# appears only when Testarossa compiles the hot loop, on the machine it is running
# on, from the instruction set it finds there. `native` is the only thing a JIT can do.
#
# It matters twice over here. OpenJ9 *silently ignores* unknown `-XX:` options --
# measured: `java -XX:CompleteNonsenseFlag=42 -version` starts happily, where HotSpot
# refuses to boot -- so a flag pretending to pin an ISA on this VM would have pinned
# exactly nothing while the label claimed otherwise. The mode says what is true instead.
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
# the timed rebuild. There is nothing to tune: scalac emits bytecode, and every
# decision that matters to this benchmark is made later, by the JIT.
#
# scalac takes several times javac's time on the same one-file kernel, and nothing
# here tries to hide that: no incremental mode, no build server, no sbt. It is a
# fact about the backend, and the Build column exists to report it.
compile_to() {
    output=$1
    mkdir -p "${output}"
    "${SCALA_HOME}/bin/scalac" -d "${output}" "${SOURCE}"
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
    # `threads` is accepted for contract compliance and deliberately ignored: scalac
    # on a single file has nothing to parallelise.
    #
    # This measures scalac, which is source -> bytecode and nothing more. The machine
    # code this program actually runs does not exist yet: OpenJ9's JIT emits it
    # during the *run*, once the loop is hot. So the Build column here is a fact
    # about scalac, not a number to rank against gcc's -- and the JIT's compile time
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
    # -- here, the JVM booting and loading both Scala libraries -- and it is a result
    # rather than overhead to be subtracted.
    #
    # No -Xshareclasses, no -Xquickstart, no -Xtune:virtualized, and no cap on vector
    # width: OpenJ9 ships a drawer full of knobs that would flatter it, and every one
    # of them is left alone. The defaults are what a Scala program gets.
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
    # This is the bytecode scalac emitted, not the machine code the JIT ends at --
    # which lives in memory, and which no tool the JDK ships will print. Compare it
    # with the Java and Kotlin rows': all three feed the same JIT, so their bytecode
    # is where the differences between them are visible.
    # Both classes, because a Scala `object` compiles to two: `Mandelbrot` holds
    # static forwarders (which is why `java Mandelbrot` works at all), and the
    # module class `Mandelbrot$` holds the actual code -- `rowIterations` among it.
    # Disassembling only the first prints a handful of forwarders and looks like a
    # kernel that vanished.
    listing=$(javap -c -p -cp "${CLASSES}:${STDLIB}" "${MAIN_CLASS}" "${MAIN_CLASS}\$")
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
