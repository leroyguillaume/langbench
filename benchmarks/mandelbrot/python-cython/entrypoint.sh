#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
BUILD_DIR=${BUILD_DIR:-/build}

# Cython emits C, and gcc compiles it, so the floating-point flags do reach a
# real compiler. They change nothing: without type annotations the generated C
# manipulates `PyFloat` objects through the C-API, never raw doubles, so there
# is no multiply-add for gcc to contract and nothing to reassociate. The three
# modes therefore produce the same checksum -- which is itself the result.
export_cflags() {
    flags="-O3"
    if [ -n "${MARCH:-}" ]; then
        flags="${flags} -march=${MARCH}"
    fi
    case "${FP_MODE:-strict}" in
    strict) flags="${flags} -ffp-contract=off" ;;
    fma) flags="${flags} -ffp-contract=fast" ;;
    fast) flags="${flags} -ffast-math" ;;
    *)
        printf 'unknown FP_MODE: %s\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac
    CFLAGS="${flags}"
    export CFLAGS
}

# The shipped extension module, built once at `docker build`.
shipped_object() {
    # The filename carries an ABI tag, e.g. mandelbrot.cpython-313-aarch64-linux-gnu.so
    find "${SOURCE_DIR}" -maxdepth 1 -name 'mandelbrot*.so' -print -quit
}

# Cython -> C -> gcc, in place. Everything it says goes to stderr.
compile_in() {
    (cd "$1" && cythonize --inplace -3 mandelbrot.py) >&2
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
  entrypoint.sh install                       cythonize the shipped module (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        disassemble the hot loop (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1
export_cflags

case "${phase}" in
install)
    compile_in "${SOURCE_DIR}"
    # The generated C and setuptools' `build/` are intermediates, not artifacts.
    # Leaving `build/` behind would ship a second copy of the object and confuse
    # anyone reading the image. The `.py` stays: the timed rebuild needs it, and
    # Python's finder prefers the extension module anyway.
    rm -rf "${SOURCE_DIR}/mandelbrot.c" "${SOURCE_DIR}/build"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # `cythonize -j` parallelises across modules, and there is exactly one.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/mandelbrot.py" "${BUILD_DIR}/mandelbrot.py"

    started=$(now_ns)
    compile_in "${BUILD_DIR}"
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped object, measured after the timer stops. Unlike a
    # pure interpreter this backend does emit machine code, so `.text` is real.
    object=$(shipped_object)
    cp "${object}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${object}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    text_bytes=$(size --format=sysv "${object}" | awk '/^\.text/ { print $2 }')

    read_cpu_time
    read_peak_memory
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # `import mandelbrot` resolves to the extension module: Python's finder tries
    # extension suffixes before source ones, so the `.so` wins over the `.py`.
    output=$(cd "${SOURCE_DIR}" && python -c 'import mandelbrot, sys; sys.exit(mandelbrot.main())' "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # Evidence, not measurement. Cython's functions are static, so the symbol is
    # mangled and absent from the dynamic table; read it out of the symtab.
    object=$(shipped_object)
    # Only a text symbol (t/T). The first name matching `_row_iterations` is
    # `__pyx_doc_..._row_iterations`, a docstring in .rodata, and objdump
    # disassembles it into an empty listing with a zero exit code.
    symbol=$(nm "${object}" | awk 'NF == 3 && $2 ~ /^[tT]$/ && $3 ~ /_row_iterations$/ { print $3; exit }')
    if [ -z "${symbol}" ]; then
        printf 'no _row_iterations text symbol in %s\n' "${object}" >&2
        exit 1
    fi
    listing=$(objdump --disassemble="${symbol}" --no-show-raw-insn "${object}")
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
