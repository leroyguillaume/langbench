#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/mandelbrot.rs
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

# The ISA baseline, translated out of gcc's spelling into rustc's.
#
# The harness speaks gcc: it hands every backend `-march=x86-64-v3` or
# `-march=armv8.2-a`. rustc speaks LLVM, and the two do not agree. On x86 LLVM
# happens to know `x86-64-v3` as a CPU; on AArch64 there is no such CPU -- the
# baseline is a *feature set*, and asking for `-C target-cpu=armv8.2-a` gets you
# "not a recognized processor (ignoring processor)" on stderr and a generic binary
# in your hand. rustc says that about `-C target-cpu=nonsense` too. It warns, it
# does not fail.
#
# So an unknown baseline dies here rather than being quietly downgraded. A silent
# fall back to generic would not break the campaign -- it would publish a row that
# claims an ISA baseline it was not compiled for, which is worse.
march_flag() {
    case "${MARCH:-}" in
    '') ;;
    x86-64-v3) printf -- '-Ctarget-cpu=x86-64-v3\n' ;;
    armv8.2-a) printf -- '-Ctarget-feature=+v8.2a\n' ;;
    *)
        printf 'unknown MARCH for rustc: %s. Add its LLVM spelling to march_flag() rather than\n' "${MARCH}" >&2
        printf 'letting rustc ignore it and hand back a generic binary.\n' >&2
        exit 1
        ;;
    esac
}

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild.
#
# `codegen-units=1` is pinned, as everywhere in this repo: the default of 16 lets
# the number of code-generation units vary with the machine, and the optimizer's
# reach varies with it. `strip=none` is pinned for the same reason -- the size
# columns describe an unstripped binary, and we strip a copy afterwards, outside
# the timer.
#
# There is no floating-point mode to choose. rustc offers no `-ffast-math` and
# LLVM may not contract an `a * b + c` that the source did not write as `mul_add`,
# so `strict` is the only semantics this backend has. That is why `bench.yaml`
# declares `strict` alone, and it is a published fact about Rust, not an omission.
compile_to() {
    output=$1
    case "${FP_MODE:-strict}" in
    strict) ;;
    *)
        printf 'unknown FP_MODE: %s (this backend distinguishes only strict)\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac

    # rustc only warns when it does not understand an ISA flag, so read its stderr
    # back and refuse the binary if it did. Belt and braces with march_flag().
    #
    # The scratch file goes to /tmp and never to BUILD_DIR. BUILD_DIR is a tmpfs at
    # run time, but this function also runs at `docker build`, as root -- and a
    # `/build` baked into the image as root-owned makes Docker mount the tmpfs with
    # *that* ownership, so the unprivileged user the container runs as could no
    # longer write to its own build directory. The directory must not exist in the
    # image at all.
    diagnostics=$(mktemp)

    # march_flag prints one flag, or nothing at all: it must split.
    # shellcheck disable=SC2046
    rustc -Copt-level=3 -Ccodegen-units=1 -Cstrip=none -Clinker=cc \
        $(march_flag) -o "${output}" "${SOURCE}" 2>"${diagnostics}" || {
        cat "${diagnostics}" >&2
        exit 1
    }
    cat "${diagnostics}" >&2

    if grep -qE 'not a recognized processor|unknown and unstable feature' "${diagnostics}"; then
        printf 'rustc ignored the ISA baseline (MARCH=%s) and compiled for generic.\n' "${MARCH:-}" >&2
        exit 1
    fi
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
    # `codegen-units=1` is pinned, and one crate with one CGU has nothing to
    # parallelise. A cargo workspace would honour it.
    mkdir -p "${BUILD_DIR}"

    started=$(now_ns)
    compile_to "${BUILD_DIR}/mandelbrot"
    elapsed_ns=$(($(now_ns) - started))

    # Sizes describe the shipped binary, measured after the timer stops. We never
    # strip during the timed build: that would add link-time work to the number.
    cp "${BINARY}" "${BUILD_DIR}/stripped"
    strip "${BUILD_DIR}/stripped"
    binary_bytes=$(stat -c %s "${BINARY}")
    binary_stripped_bytes=$(stat -c %s "${BUILD_DIR}/stripped")
    # Only .text is comparable across implementations: total file size measures
    # linking policy, not codegen -- and Rust's policy is to link std statically,
    # which is exactly the kind of thing the Binary column would mistake for code.
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
    # Evidence, not measurement. At -O3 with one codegen unit, nothing named
    # `row_iterations` survives: it inlines into the worker closure, which inlines
    # into `std::sys::backtrace::__rust_begin_short_backtrace` -- the wrapper std
    # wraps every spawned thread in so that backtraces stop at the thread entry.
    # That wrapper *is* this backend's `work`, the same way C's thread function is,
    # so it is what there is to read. Prefer the named function if a future rustc
    # ever leaves it standing.
    symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
        | grep -m1 'row_iterations' || true)
    if [ -z "${symbol}" ]; then
        symbol=$(nm "${BINARY}" | awk 'NF == 3 && $2 ~ /^[tT]$/ { print $3 }' \
            | grep -m1 '__rust_begin_short_backtrace' || true)
    fi
    if [ -z "${symbol}" ]; then
        printf 'neither row_iterations nor the thread entry survives in %s\n' "${BINARY}" >&2
        exit 1
    fi
    # Not `--demangle`: objdump matches --disassemble= against the *mangled* name,
    # and asking it to demangle at the same time makes the match fail and the
    # listing come back empty. Rust's mangling is legible enough to read as-is.
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
