#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# PyPy has exactly one floating-point semantics. Its tracing JIT emits machine
# code, but it is not licensed to change the arithmetic while doing so: it never
# contracts a multiply-add into an FMA, and it never reassociates. So the three
# modes produce the same trace and, necessarily, the same checksum -- which is
# itself the result, and it is CPython's checksum, and C's.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: PyPy has one FP semantics; mode %s behaves exactly like strict\n' \
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
    pypy3 -m compileall -q "${SOURCE_DIR}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # `compileall` on a single module has nothing to parallelise.
    #
    # PyPy's ahead-of-run "build" is source -> bytecode, exactly as CPython's is.
    # The machine code -- the thing PyPy is famous for -- is emitted by the tracing
    # JIT during the *run*, once a loop is hot enough to be worth compiling, and it
    # never lands in a file. So this column measures PyPy's parser, and the JIT's
    # compile time is billed to the run column, where it happened.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/mandelbrot.py" "${BUILD_DIR}/mandelbrot.py"

    started=$(now_ns)
    # -f: never trust a cached .pyc; the point is to measure the compile.
    pypy3 -m compileall -q -f "${BUILD_DIR}/mandelbrot.py" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. A .pyc is bytecode,
    # and putting its size next to an ELF's would rank packaging, not codegen.
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # `import`, not `-m`: the shipped __pycache__ is used either way, but `-m`
    # drags in `runpy`, and that cost would land in the Startup column. The
    # `python-cpython` and `python-cython` backends import too, so the three rows
    # stay comparable.
    output=$(cd "${SOURCE_DIR}" && pypy3 -c 'import mandelbrot, sys; sys.exit(mandelbrot.main())' "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    # This is the bytecode PyPy starts from, not the machine code its JIT ends at
    # -- that lives in memory, keyed by trace, and there is no file to read it out
    # of. Compare it with `python-cpython`'s: the two are near enough identical,
    # and every difference between those two rows is what happens *after* this.
    pypy3 -m dis "${SOURCE_DIR}/mandelbrot.py"
    ;;

*)
    usage
    ;;
esac
