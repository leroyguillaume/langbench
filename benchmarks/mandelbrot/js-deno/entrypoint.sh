#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
KERNEL=mandelbrot.mjs
BUILD_DIR=${BUILD_DIR:-/build}

# Deno's permissions are deny-by-default, and the kernel needs exactly two grants:
# read, because a worker loads its own module file, and env, because it reads
# `process.argv`. Not `-A`: an allow-all flag would let a future edit reach the
# network without anyone noticing, and `--network=none` should not be the only
# thing standing between this benchmark and a download.
PERMISSIONS="--allow-read --allow-env"

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
  entrypoint.sh install                       warm DENO_DIR and check the module (image build only)
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
    # Runs at `docker build`, where the network still exists: this is what pulls
    # the node typings into DENO_DIR so the timed build can check offline.
    deno check "${SOURCE_DIR}/${KERNEL}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # checking one module has nothing to parallelise.
    #
    # `deno check` is Deno's ahead-of-run step, and it is a *type check* -- strictly
    # more work than `node --check`, which only parses. That is not a thumb on the
    # scale, it is the difference between the two runtimes: read this column
    # against Deno's own numbers, never across the three JavaScript rows.
    #
    # The copy in the tmpfs is what makes the number real. Deno caches check
    # results by module specifier, so re-checking the shipped path would find the
    # answer already there and report an instant build -- the Go trap. A fresh path
    # in an empty tmpfs is cold for our module while DENO_DIR stays warm for the
    # typings, which is exactly the split we want.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/${KERNEL}" "${BUILD_DIR}/${KERNEL}"

    started=$(now_ns)
    deno check "${BUILD_DIR}/${KERNEL}" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. There is no binary,
    # and a zero would be a claim about one.
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # No `--check` here, deliberately: `deno run` strips types and executes, and
    # type checking is the build phase's job. Checking again on every run would
    # bill the run column for work the build column already reported.
    #
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`.
    # The gap between this and the harness's external clock is runtime startup
    # cost, and it is a result rather than overhead to be subtracted.
    # PERMISSIONS is a list of flags, not one word: it must split.
    # shellcheck disable=SC2086
    output=$(cd "${SOURCE_DIR}" && deno run ${PERMISSIONS} "${KERNEL}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # The same V8 Ignition bytecode `js-nodejs` prints, reached through Deno's
    # --v8-flags, which is itself the point -- two runtimes, one engine.
    # PERMISSIONS is a list of flags, not one word: it must split.
    # shellcheck disable=SC2086
    listing=$(cd "${SOURCE_DIR}" && deno run ${PERMISSIONS} \
        --v8-flags=--print-bytecode,--print-bytecode-filter=rowIterations \
        "${KERNEL}" 8 8 1)
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
