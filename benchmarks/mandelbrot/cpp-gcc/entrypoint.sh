#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/mandelbrot.cpp
# The binary the image ships, compiled once at `docker build`. This is what
# `run` executes and what the reported sizes describe.
BINARY=/usr/local/bin/mandelbrot
# A tmpfs at run time: the timed rebuild writes here, and the container's fresh
# writable layer means it is empty on every invocation.
BUILD_DIR=${BUILD_DIR:-/build}
CXX=g++

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

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild. The same flags as `c-gcc` plus a language standard: the two
# rows are meant to differ in the language, not in the optimizer's instructions.
compile_to() {
    output=$1
    set -- -O3 -std=c++20 -pthread -Wall -Wextra
    if [ -n "${MARCH:-}" ]; then
        set -- "$@" "-march=${MARCH}"
    fi
    case "${FP_MODE:-strict}" in
    # GCC contracts into FMA by default in C++ as in C, so `strict` says so.
    strict) set -- "$@" -ffp-contract=off ;;
    fma) set -- "$@" -ffp-contract=fast ;;
    fast) set -- "$@" -ffast-math ;;
    *)
        printf 'unknown FP_MODE: %s\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac
    "${CXX}" "$@" -o "${output}" "${SOURCE}"
}

# The hot loop's symbol, whatever the compiler decided to call it.
#
# The C kernel disassembles `work`, which survives -O3 because pthread_create
# takes its address. C++ has no such function: the worker is a lambda, and
# libstdc++ wraps it in a `std::thread::_State_impl<...>::_M_run()` whose address
# goes into the thread. `row_iterations` is inlined into *that*, so that is what
# there is to read. The name is mangled and template-laden, so find it rather than
# spell it.
hot_symbol() {
    nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tTwW]$/ { print $3 }' | grep -m1 '_M_run'
}

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       compile the shipped binary (image build only)
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
    compile_to "${BINARY}"
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored:
    # this kernel is a single translation unit, so there is nothing for `-j` to
    # parallelise.
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
    printf '{"phase":"build","elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s}\n' \
        "${elapsed_ns}" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}"
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
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}"
    ;;

disasm)
    # Evidence, not measurement. When one compiler is 3x faster than another we
    # look for the `vmulpd` instead of speculating about the vectorizer.
    symbol=$(hot_symbol) || true
    if [ -z "${symbol:-}" ]; then
        printf 'no thread-entry symbol in %s: the hot loop is somewhere else now\n' "${BINARY}" >&2
        exit 1
    fi
    # Not `--demangle`: objdump matches --disassemble= against the *mangled* name,
    # and asking it to demangle at the same time makes the match fail and the
    # listing come back empty. Demangle the output afterwards, with c++filt.
    listing=$(objdump --disassemble="${symbol}" --no-show-raw-insn "${BINARY}" | c++filt)
    # An empty listing is the silent failure this guard exists to catch.
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
