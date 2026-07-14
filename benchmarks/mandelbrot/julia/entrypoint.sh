#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE_DIR=/usr/local/src/mandelbrot
KERNEL=mandelbrot.jl
BUILD_DIR=${BUILD_DIR:-/build}

# The argument types of the hot loop, spelled once. `precompile` needs them: it
# compiles a *method for a signature*, which is what Julia's compiler is actually
# organised around.
ROW_SIGNATURE='(Int, Int, Int, Float64, Float64)'

# Floating-point is strict here, in every mode, and there is nothing to pass to make
# it so: Julia never contracts a multiply-add into an FMA on its own -- fusing is
# `muladd`, which the source has to ask for -- and it never reassociates.
# (`--math-mode=fast` existed once; it was deprecated because it was unsound across
# function boundaries, and in 1.11 it does nothing.) So the checksum is bit-identical
# whatever the ISA target, which is exactly what makes it a gate on the ISA target.

# The ISA target the JIT compiles for -- the whole of the mode, in one word.
#
# `MARCH` carries it: the campaign's pinned baseline (`x86-64-v3`, `armv8.2-a`), or
# the literal `native`. Julia speaks LLVM's CPU names, and `native` is one of them,
# so both arrive ready to use and there is nothing here to translate.
#
# Julia's default is `native`, which is why the target is *always* passed explicitly,
# including -- especially -- in the baseline mode. A forgotten flag here does not
# fail: it compiles for the bench machine and publishes the result in the baseline
# column, where every number would be internally consistent and wrong.
#
# The empty `MARCH` is the machine whose baseline the harness does not know. `generic`
# is the honest target for it -- LLVM's own floor for the architecture -- and it is
# what the row then reports getting, because that is what it got.
cpu_target() {
    printf '%s\n' "${MARCH:-generic}"
}

cpu_target_flag() {
    printf -- '--cpu-target=%s\n' "$(cpu_target)"
}

# What the JIT was actually told to target, echoed back so a sample can say it. The
# mode says what was *asked for*; this says what was got, and Julia is the backend
# where the difference is worth watching: it is a JIT, and a JIT that is not told
# otherwise compiles for the machine under it.
#
# It prints its own trailing comma: the caller splices it into a JSON object that has
# to stay well-formed without it.
isa_json() {
    printf '"isa":"%s",' "$(cpu_target)"
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
  entrypoint.sh install                       compile the kernel once (image build only)
  entrypoint.sh build <threads>               timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                        emit the hot loop's machine code (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1

case "${phase}" in
install)
    # Proves the kernel parses and compiles inside the image. Nothing is kept: a
    # Julia depot caches compiled code for *packages*, and this is a script, so
    # every run pays the compile again. That is the fact this row publishes, and
    # caching it away in the image would be measuring a program nobody runs.
    julia "$(cpu_target_flag)" -e \
        "include(\"${SOURCE_DIR}/${KERNEL}\"); precompile(row_iterations, ${ROW_SIGNATURE}) || error(\"row_iterations did not compile\")"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored: one
    # method's codegen is not parallel.
    #
    # This is Julia's compile step, run to completion and nothing else: parse the
    # file, infer the types, hand the hot loop to LLVM. `include` does not execute
    # the workload -- the kernel guards on PROGRAM_FILE for exactly this reason --
    # so what is timed is compilation, which is what the Build column means
    # everywhere else in the table.
    #
    # The same work happens again inside every measured run, because a script's
    # compiled code is not cached anywhere. Julia is the one backend whose build
    # column and run column overlap, and that is the honest picture of a JIT.
    mkdir -p "${BUILD_DIR}"
    cp "${SOURCE_DIR}/${KERNEL}" "${BUILD_DIR}/${KERNEL}"

    started=$(now_ns)
    julia "$(cpu_target_flag)" -e \
        "include(\"${BUILD_DIR}/${KERNEL}\"); precompile(row_iterations, ${ROW_SIGNATURE}) || error(\"row_iterations did not compile\")" >&2
    elapsed_ns=$(($(now_ns) - started))

    read_cpu_time
    read_peak_memory
    # No artifact on disk: the sizes are null, not zero. Julia's native code lives
    # in memory, and a zero would be a claim about a file that does not exist.
    printf '{"phase":"build","elapsed_ns":%s,%s"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "$(isa_json)" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # `--threads` is what gives Julia's scheduler somewhere to put the tasks; the
    # kernel is told the same number on argv and spawns exactly that many. It never
    # asks the machine, because a runtime that reads the cgroup quota its own way
    # would quietly disagree with the harness.
    #
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`. Its
    # clock starts before the first call, so the JIT's compile time is inside the
    # number -- that is not a flaw in the measurement, it is what a JIT costs.
    output=$(cd "${SOURCE_DIR}" \
        && julia "$(cpu_target_flag)" --threads="$4" "${KERNEL}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,%s"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "$(isa_json)" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # Evidence, not measurement -- and the real thing, not bytecode: `code_native`
    # prints the machine code LLVM generated for this signature. The one backend in
    # the table that JITs *and* will show you the instructions. Read it next to
    # c-gcc's `objdump` output; it is the same kind of listing.
    # `using InteractiveUtils`: code_native is not in Base. A bare `julia -e` gets a
    # Main without it, and the REPL's habit of having it already there is exactly
    # the sort of thing that works interactively and fails in a container.
    listing=$(julia "$(cpu_target_flag)" -e \
        "using InteractiveUtils; include(\"${SOURCE_DIR}/${KERNEL}\"); code_native(stdout, row_iterations, ${ROW_SIGNATURE}; syntax = :att)")
    if ! printf '%s\n' "${listing}" | grep -qE '^[[:space:]]*[a-z]'; then
        printf 'empty listing for row_iterations: LLVM emitted nothing\n' >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
