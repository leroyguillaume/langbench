#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
KERNEL=mandelbrot.mjs
BUILD_DIR=${BUILD_DIR:-/build}

# The ISA this row reports, and it is not a choice: V8's TurboFan emits machine
# code for the CPU it is running on, once the loop is hot, and there is no flag
# that would make it emit code for a lesser one. A JIT cannot be portable across
# machines it will never see -- it only ever sees this one. So the answer to "which
# instruction set did this code get?" is the same word an ahead-of-time backend
# reports when it was *asked* to target the host, and the coincidence is the
# finding: one had to ask, the other could not refuse.
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

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       check the shipped module (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        dump the hot loop's bytecode (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1

case "${phase}" in
install)
    node --check "${SOURCE_DIR}/${KERNEL}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # parsing one module has nothing to parallelise.
    #
    # `node --check` is what Node does ahead of a run and no more: it parses the
    # module and compiles it, without executing it. V8 is a JIT, so the machine
    # code this program eventually runs does not exist yet and will not exist
    # until the hot loop is warm -- which is why the run phase, not this one, is
    # where a JIT backend spends its compile time. The Build column here is a fact
    # about Node, not a number to rank against gcc's.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/${KERNEL}" "${BUILD_DIR}/${KERNEL}"

    started=$(now_ns)
    node --check "${BUILD_DIR}/${KERNEL}" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. There is no binary,
    # and a zero would be a claim about one.
    printf '{"phase":"build","elapsed_ns":%s,"isa":"%s","user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${ISA}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`.
    # The gap between this and the harness's external clock is runtime startup
    # cost -- here, V8 booting an isolate -- and it is a result rather than
    # overhead to be subtracted.
    output=$(cd "${SOURCE_DIR}" && node "${KERNEL}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":"%s","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${ISA}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # V8's Ignition bytecode, not the machine code TurboFan eventually emits --
    # that is JIT-compiled, tier by tier, and never lands in a file we could read.
    # A tiny grid, because the point is to compile the function, not to run it.
    listing=$(cd "${SOURCE_DIR}" \
        && node --print-bytecode --print-bytecode-filter=rowIterations "${KERNEL}" 8 8 1)
    if ! printf '%s\n' "${listing}" | grep -q 'Bytecode'; then
        printf 'no bytecode for rowIterations: V8 never compiled it\n' >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
