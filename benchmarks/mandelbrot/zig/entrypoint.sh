#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/mandelbrot.zig
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

# The ISA target, translated out of gcc's spelling into Zig's.
#
# The harness speaks gcc: it hands every backend `-march=x86-64-v3`,
# `-march=armv8.2-a` or `-march=native`. Zig spells the first with underscores,
# expresses the second as a feature added to a generic CPU, and calls the third
# `-mcpu=native` -- the same word, a different flag. An unknown target dies here
# rather than being silently downgraded to `-mcpu=baseline`: a row that claims an
# ISA it was not compiled for is worse than a row that is missing.
mcpu_flag() {
    case "${MARCH:-}" in
    '') ;;
    x86-64-v3) printf -- '-mcpu=x86_64_v3\n' ;;
    armv8.2-a) printf -- '-mcpu=generic+v8_2a\n' ;;
    native) printf -- '-mcpu=native\n' ;;
    *)
        printf 'unknown MARCH for zig: %s. Add its Zig spelling to mcpu_flag() rather than\n' "${MARCH}" >&2
        printf 'letting the build fall back to a baseline nobody asked for.\n' >&2
        exit 1
        ;;
    esac
}

# What the compiler was actually given, echoed back so that a sample can say it.
# The mode says what was *asked for*; this says what was got. It is the `MARCH`
# that reached mcpu_flag() and nothing else -- not the Zig spelling it came back as,
# which is this backend's private business, and not anything read off the host.
#
# An empty `MARCH` means zig fell back to its own default target and told nobody
# which, so the field is omitted rather than guessed at. It prints its own trailing
# comma: the caller splices it into a JSON object that has to stay well-formed
# without it.
isa_json() {
    if [ -n "${MARCH:-}" ]; then
        printf '"isa":"%s",' "${MARCH}"
    fi
}

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild.
#
# `--cache-dir` is the *local* cache -- our module -- and every caller points it
# somewhere cold. `ZIG_GLOBAL_CACHE_DIR` is the toolchain cache and stays warm
# across runs; see the Dockerfile for why that split is the honest one.
#
# Floating-point is strict here, in every mode, and no flag says so. Zig's float
# mode is `.strict` unless the source says `@setFloatMode(.optimized)` -- a
# statement in the program, not a flag on the compiler -- and this kernel does not
# say it. `-OReleaseFast` is an optimization level and touches none of that: it is
# named after speed, not after the arithmetic, which is exactly the confusion worth
# writing down here. The checksum the run reports is the checksum the source asks
# for, in both modes.
compile_to() {
    output=$1
    cache=$2
    # mcpu_flag prints one flag, or nothing at all: it must split.
    # shellcheck disable=SC2046
    zig build-exe -OReleaseFast $(mcpu_flag) \
        --cache-dir "${cache}" \
        -femit-bin="${output}" \
        "${SOURCE}"
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
    # The local cache is a throwaway: what must survive into the image is the
    # global one, which this compile also populates with compiler_rt and std.
    compile_to "${BINARY}" /tmp/zig-install-cache
    rm -rf /tmp/zig-install-cache
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored: zig
    # parallelises its own build internally and takes no -j here.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    # A cache in the tmpfs, which is empty on every invocation: our module is
    # compiled from scratch, while the warm global cache keeps std out of the
    # number. Without this, zig would find the shipped binary already built and
    # the Build column would read "instant", which is not a fact about Zig.
    compile_to "${BUILD_DIR}/mandelbrot" "${BUILD_DIR}/zig-cache" >&2
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped binary, measured after the timer stops. We never
    # strip during the timed build: that would add link-time work to the number.
    cp "${BINARY}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${BINARY}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    # Only .text is comparable across implementations: total file size measures
    # linking policy, not codegen -- and Zig's policy here is a static binary with
    # no libc at all, which the Binary column would otherwise read as bloat.
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
    # Evidence, not measurement. At ReleaseFast neither `rowIterations` nor
    # `Worker.run` survives as a symbol: both inline into the thread entry that
    # std.Thread hands to clone(), which is this backend's `work`. Zig keeps its
    # symbols legible, so no demangling is needed -- but the entry's name carries a
    # generated instantiation number, so match on the stable part.
    symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
        | grep -m1 'rowIterations' || true)
    if [ -z "${symbol}" ]; then
        symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
            | grep -m1 'entryFn' || true)
    fi
    if [ -z "${symbol}" ]; then
        printf 'neither rowIterations nor the thread entry survives in %s\n' "${BINARY}" >&2
        exit 1
    fi
    listing=$(objdump --disassemble="${symbol}" --no-show-raw-insn "${BINARY}")
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
