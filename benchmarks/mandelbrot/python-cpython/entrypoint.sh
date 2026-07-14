#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
# The bytecode the image ships, compiled once at `docker build`. `run` executes
# it; the timed rebuild below produces a throwaway copy.
BUILD_DIR=${BUILD_DIR:-/build}

# Floating-point is strict here, and there is nothing to pass to make it so: the
# interpreter evaluates one binary operator at a time, so it has nothing to contract
# a multiply-add out of and nothing to reassociate. There was never a knob, and
# removing the mode axis removed no flag.

# The ISA this row actually got, which is the one thing about this backend that the
# mode alone cannot say.
#
# The mode is `baseline`, because CPython is neither compiled ahead of the run nor
# JIT-compiled during it -- 3.13's JIT is experimental, off by default, and not built
# into this image -- so no machine code is ever generated for *this* CPU. The machine
# code that runs the hot loop is CPython's own eval loop, and it was compiled when the
# image was packaged, by somebody who had to boot on every machine there is: no
# `-march` appears in the interpreter's own CFLAGS or in its configure line, so the
# eval loop was built for the toolchain's floor -- plain `x86-64` (v1), `armv8-a` --
# which is *below* the baseline every compiled row in this campaign was held to.
#
# So the row cannot report the campaign's baseline: it never got it, and echoing back
# the level it was handed is precisely the silent lie the `isa` field exists to kill.
# It reports `distro`: the packager chose this ISA, this campaign did not, and no
# `-march` was recorded for it to name. That is a fact about *who chose*, and it stays
# true whatever the packager chooses next.
#
# It is a constant and not a `sysconfig` lookup on purpose. Asking the interpreter
# would mean starting a second interpreter inside a measured container, and the gap
# between the external clock and the kernel's own `elapsed_ns` is a published column:
# a run that measured itself would be a run that changed itself. The lookup belongs in
# the comment above, where it has already been done.
#
# It prints its own trailing comma: the caller splices it into a JSON object that has
# to stay well-formed without it.
isa_json() {
    printf '"isa":"distro",'
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
    read_peak_memory
    # No machine-code artifact: the sizes are null, not zero. A .pyc is bytecode,
    # and putting its size next to an ELF's would rank packaging, not codegen.
    printf '{"phase":"build","elapsed_ns":%s,%s"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "$(isa_json)" "${user_usec}" "${system_usec}" "${peak_bytes}"
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
    read_peak_memory
    printf '{"phase":"run","checksum":%s,%s"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "$(isa_json)" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # The analogue of `objdump` for a bytecode backend: evidence, not measurement.
    python -m dis "${SOURCE_DIR}/mandelbrot.py"
    ;;

*)
    usage
    ;;
esac
