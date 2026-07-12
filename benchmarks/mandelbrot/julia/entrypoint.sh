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

# Julia has exactly one floating-point semantics here. It never contracts a
# multiply-add into an FMA on its own -- fusing is `muladd`, which the source has
# to ask for -- and it never reassociates. (`--math-mode=fast` existed once; it was
# deprecated because it was unsound across function boundaries, and in 1.11 it does
# nothing.) So the three modes produce the same native code and, necessarily, the
# same checksum -- which is itself the result.
check_fp_mode() {
    case "${FP_MODE:-strict}" in
    strict) ;;
    fma | fast)
        printf 'note: Julia has one FP semantics; mode %s behaves exactly like strict\n' \
            "${FP_MODE}" >&2
        ;;
    *)
        printf 'unknown FP_MODE: %s\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac
}

# The ISA baseline. Julia's default is `native`, which this project forbids: the
# JIT would compile for whatever CPU the bench machine happens to have, and the
# baseline would silently vary with the host. So it is always passed explicitly.
#
# Julia speaks LLVM's CPU names and validates them -- `--cpu-target=nonsense` is a
# hard error, not a warning -- so an unknown baseline fails the campaign here
# rather than quietly producing a generic binary, which is more than rustc can say.
cpu_target_flag() {
    if [ -z "${MARCH:-}" ]; then
        printf -- '--cpu-target=generic\n'
    else
        printf -- '--cpu-target=%s\n' "${MARCH}"
    fi
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
check_fp_mode

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
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":null,"binary_stripped_bytes":null,"text_bytes":null,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
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
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
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
