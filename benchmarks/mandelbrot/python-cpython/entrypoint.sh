#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
# The bytecode the image ships, compiled once at `docker build`. `run` executes
# it; the timed rebuild below produces a throwaway copy.
BUILD_DIR=${BUILD_DIR:-/build}

# CPython has exactly one floating-point semantics. There is no `-ffp-contract`
# to turn off and no `-ffast-math` to turn on: the interpreter never contracts a
# multiply-add, and it never reassociates. So the three modes produce the same
# bytecode and, necessarily, the same checksum -- which is itself the result.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: CPython has one FP semantics; mode %s behaves exactly like strict\n' \
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
  entrypoint.sh install                       compile the shipped bytecode (image build only)
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
    # `compileall` only has short options; `--quiet` makes argparse exit 2.
    python -m compileall -q "${SOURCE_DIR}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # `compileall` on a single module has nothing to parallelise.
    #
    # CPython's "build" is source -> bytecode. It is milliseconds rather than
    # seconds, and it is not comparable to a gcc invocation; the report's Build
    # column is a fact about the backend, not a ranking across categories.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/mandelbrot.py" "${BUILD_DIR}/mandelbrot.py"

    started=$(now_ns)
    # -f: never trust a cached .pyc; the point is to measure the compile.
    python -m compileall -q -f "${BUILD_DIR}/mandelbrot.py" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    # No machine-code artifact: the sizes are null, not zero. A .pyc is bytecode,
    # and putting its size next to an ELF's would rank packaging, not codegen.
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # `import`, not `-m`: the shipped __pycache__ is used either way, but `-m`
    # drags in `runpy`, and that cost would land in the Startup column. The
    # `python-cython` backend imports too, so the two rows stay comparable.
    # A script run by path would be recompiled on every invocation, and that
    # cost belongs to the build phase, not here.
    output=$(cd "${SOURCE_DIR}" && python -c 'import mandelbrot, sys; sys.exit(mandelbrot.main())' "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    python -m dis "${SOURCE_DIR}/mandelbrot.py"
    ;;

*)
    usage
    ;;
esac
