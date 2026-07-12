#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
KERNEL=mandelbrot.mts
BUILD_DIR=${BUILD_DIR:-/build}

# JavaScriptCore has exactly one floating-point semantics, because ECMAScript
# gives it no choice: every arithmetic operator's result is specified, an engine
# may not contract a multiply-add into an FMA, and it may not reassociate. So the
# three modes produce the same code and, necessarily, the same checksum -- which
# is itself the result, and it is the same checksum V8 produces.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: JavaScriptCore has one FP semantics; mode %s behaves exactly like strict\n' \
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

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       bundle the shipped module (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        dump the module's bytecode (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1
check_fp_mode

case "${phase}" in
install)
    bun build "${SOURCE_DIR}/${KERNEL}" --target=bun --outfile=/tmp/bundle.js >&2
    rm -f /tmp/bundle.js
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # bundling one module has nothing to parallelise.
    #
    # `bun build` is Bun's ahead-of-run step: it transpiles and bundles. Node
    # parses (`node --check`), Deno type-checks (`deno check`), Bun bundles --
    # three runtimes, three different notions of "before the run", and the Build
    # column reports each one honestly rather than inventing a common denominator
    # that none of them actually performs. Read it against Bun's own numbers.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/${KERNEL}" "${BUILD_DIR}/${KERNEL}"

    started=$(now_ns)
    bun build "${BUILD_DIR}/${KERNEL}" --target=bun --outfile="${BUILD_DIR}/bundle.js" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. The bundle is
    # JavaScript, and putting its size next to an ELF's would rank packaging, not
    # codegen.
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # The kernel, not the bundle: the bundle is a build artifact, and every other
    # row here runs what the image ships. Bun loads the `.mjs` directly, which is
    # also what a Bun user would do.
    #
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`.
    # The gap between this and the harness's external clock is runtime startup
    # cost, and it is a result rather than overhead to be subtracted.
    output=$(cd "${SOURCE_DIR}" && bun "${KERNEL}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # JavaScriptCore's bytecode, where the V8 rows print Ignition's -- reading the
    # two side by side is the point of having both engines in the table.
    #
    # The whole module, not one function: JSC's dump takes no filter, so this is
    # every function it compiled. Pipe it through `grep -A` for the hot loop.
    listing=$(cd "${SOURCE_DIR}" \
        && BUN_JSC_dumpGeneratedBytecodes=1 bun "${KERNEL}" 8 8 1 2>&1)
    if ! printf '%s\n' "${listing}" | grep -q 'rowIterations'; then
        printf 'no bytecode for rowIterations: JavaScriptCore never compiled it\n' >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
