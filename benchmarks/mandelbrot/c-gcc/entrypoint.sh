#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/mandelbrot.c
# The binary the image ships, compiled once at `docker build`. This is what
# `run` executes and what the reported sizes describe.
BINARY=/usr/local/bin/mandelbrot
# A tmpfs at run time: the timed rebuild writes here, and the container's fresh
# writable layer means it is empty on every invocation.
BUILD_DIR=${BUILD_DIR:-/build}

now_ns() {
    date +%s%N
}

# CPU time comes from the cgroup, never from the harness's `rusage`: the
# workload runs in a different process tree from the `docker` client.
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

# What the compiler was actually given, echoed back so that a sample can say it.
# The mode says what was *asked for*; this says what was got. It is the `MARCH`
# that reached the command line and nothing else: reading the ISA back off the
# binary, or off /proc/cpuinfo, would be a second source of truth about the one
# thing the mode exists to pin.
#
# An empty `MARCH` means the compiler chose a target and told nobody which, so the
# field is omitted rather than guessed at. It prints its own trailing comma: the
# caller splices it into a JSON object that has to stay well-formed without it.
isa_json() {
    if [ -n "${MARCH:-}" ]; then
        printf '"isa":"%s",' "${MARCH}"
    fi
}

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild. The axis is the ISA target, not "optimization on or off":
# every mode is -O3, and the modes differ in `-march` alone.
compile_to() {
    output=$1
    # `-ffp-contract=off` is unconditional, in every mode, forever. GCC contracts
    # `a * b + c` into an FMA by default in C, and an FMA rounds once where the
    # source says twice: a contracted build computes a *different* checksum from an
    # uncontracted one. The checksum is how a run is known to be right, so the
    # arithmetic is a constraint of the workload and not a knob of the build. No
    # mode relaxes it, and `-ffast-math` is spelled nowhere in this repository.
    set -- -O3 -std=c11 -pthread -Wall -Wextra -ffp-contract=off
    # `MARCH` carries the whole mode: a pinned baseline (`x86-64-v3`, `armv8.2-a`)
    # or the literal `native`. gcc's spelling is the one the harness speaks, so
    # both arrive ready to use and there is nothing here to translate.
    if [ -n "${MARCH:-}" ]; then
        set -- "$@" "-march=${MARCH}"
    fi
    gcc "$@" -o "${output}" "${SOURCE}"
}

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                     compile the shipped binary (image build only)
  entrypoint.sh build <threads>             timed rebuild from a clean tree
  entrypoint.sh run <n> <max_iter> <threads>  timed execution
  entrypoint.sh disasm                      disassemble the hot loop (not part of the contract)
EOF
    exit 2
}

[ "$#" -ge 1 ] || usage
phase=$1

case "${phase}" in
install)
    compile_to "${BINARY}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # this kernel is a single translation unit, so there is nothing for `-j` to
    # parallelise. A language whose build does parallelise must honour it.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    compile_to "${BUILD_DIR}/mandelbrot" >&2
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped binary, measured after the timer stops. We never
    # strip during the timed build: that would add link-time work to the number.
    cp "${BINARY}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${BINARY}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    # Only .text is comparable across implementations: total file size measures
    # linking policy, not codegen.
    text_bytes=$(size --format=sysv "${BINARY}" | awk '/^\.text/ { print $2 }')

    read_cpu_time
    read_peak_memory
    printf '{"phase":"build","elapsed_ns":%s,%s"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "$(isa_json)" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}" "${peak_bytes}"
    ;;

run)
    [ "$#" -eq 4 ] || usage
    # The program self-times its hot loop and prints `<checksum> <elapsed_ns>`.
    # The gap between this and the harness's external clock is runtime startup
    # cost, which is a result rather than overhead to be subtracted.
    output=$("${BINARY}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    read_peak_memory
    printf '{"phase":"run","checksum":%s,%s"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "$(isa_json)" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
    ;;

disasm)
    # Evidence, not measurement. When one compiler is 3x faster than another we
    # look for the `vmulpd` instead of speculating about the vectorizer.
    #
    # `work` and not `row_iterations`: at -O3 the latter is inlined and its
    # symbol is gone, so asking for it prints an empty listing and exits 0.
    listing=$(objdump --disassemble=work --no-show-raw-insn "${BINARY}")
    # An empty listing is the silent failure this guard exists to catch.
    if ! printf '%s\n' "${listing}" | grep -qE '^[[:space:]]+[0-9a-f]+:'; then
        printf 'empty listing for work: the symbol is missing or is not code\n' >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
