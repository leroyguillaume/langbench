#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
KERNEL=mandelbrot.mts
BUILD_DIR=${BUILD_DIR:-/build}

# V8 has exactly one floating-point semantics, because ECMAScript gives it no
# choice: every arithmetic operator's result is specified, an engine may not
# contract a multiply-add into an FMA, and it may not reassociate. So the three
# modes produce the same code and, necessarily, the same checksum -- which is
# itself the result.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: V8 has one FP semantics; mode %s behaves exactly like strict\n' \
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
check_fp_mode

case "${phase}" in
install)
    node --check "${SOURCE_DIR}/${KERNEL}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # parsing one module has nothing to parallelise.
    #
    # `node --check` is what Node does ahead of a run and no more: it strips the
    # types, parses the module and compiles it, without executing it. It does not
    # *type-check* -- Node erases annotations, it never reads them -- so this
    # number and js-nodejs's measure very nearly the same work, which is the point. V8 is a JIT, so the machine
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
    # No machine-code artifact: the sizes are null, not zero. There is no binary,
    # and a zero would be a claim about one.
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}"
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
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}"
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
