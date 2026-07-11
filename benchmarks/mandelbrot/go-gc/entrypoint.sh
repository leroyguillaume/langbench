#!/bin/sh
# Container contract: exactly one JSON object on stdout, everything else on
# stderr. See METHODOLOGY.md#container-contract.
set -eu

SOURCE=/usr/local/src/mandelbrot/mandelbrot.go
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

# The ISA baseline, translated out of gcc's spelling into Go's.
#
# Go does not take a `-march`. It takes a per-architecture environment variable
# with its own vocabulary of levels, and it silently ignores the ones that do not
# belong to the architecture it is building for -- so `GOAMD64=v3` on arm64 is not
# an error, it is a no-op, and the row would claim a baseline it never had. An
# unknown baseline therefore dies here.
export_march() {
    case "${MARCH:-}" in
    '') ;;
    x86-64-v3)
        GOAMD64=v3
        export GOAMD64
        ;;
    armv8.2-a)
        GOARM64=v8.2
        export GOARM64
        ;;
    *)
        printf 'unknown MARCH for go: %s. Add its GOAMD64/GOARM64 spelling to export_march()\n' "${MARCH}" >&2
        printf 'rather than letting go build for a baseline nobody asked for.\n' >&2
        exit 1
        ;;
    esac
}

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild.
#
# There is no floating-point mode to choose, and Go is the one language in this
# table where that is a statement about the *source* rather than the compiler. The
# spec lets gc fuse a multiply-add whenever it likes; the kernel forbids it with
# explicit float64() rounding points, because the strict-mode checksum is not
# negotiable. A fused build would be a different program -- not a flag away -- so
# this backend distinguishes `strict` alone. Read the header of mandelbrot.go: the
# unrounded version really does return a different checksum, on this very machine.
compile_to() {
    output=$1
    case "${FP_MODE:-strict}" in
    strict) ;;
    *)
        printf 'unknown FP_MODE: %s (this backend distinguishes only strict)\n' "${FP_MODE:-}" >&2
        exit 1
        ;;
    esac
    export_march
    # -trimpath: the source path is baked into the binary otherwise, and the shipped
    # binary and the rebuilt one would differ in bytes that are not code.
    go build -trimpath -o "${output}" "${SOURCE}"
}

usage() {
    cat >&2 <<'EOF'
usage:
  entrypoint.sh install                       compile the shipped binary (image build only)
  entrypoint.sh warm-cache                    empty GOCACHE, then refill it with std (image build only)
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

warm-cache)
    # The Go trap, disarmed. See the Dockerfile: `install` filled GOCACHE with our
    # package, which would make the timed build an instant cache hit. Throw the
    # whole cache away and put back only what a Go developer would already have --
    # a compiled standard library.
    #
    # Without the refill we would be timing "Go compiles its own std" against
    # "Rust ships a precompiled one", which is a fact about our Dockerfile and not
    # about either language.
    export_march
    go clean -cache
    go build std
    ;;

build)
    [ "$#" -eq 2 ] || usage
    # `threads` is accepted for contract compliance and deliberately ignored: `go
    # build` parallelises across packages, and a single-file main package is one
    # package. It honours the CPU count it finds, which is the harness's `--cpu`.
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
    # linking policy, not codegen -- and Go's policy is a static binary carrying a
    # garbage collector and a scheduler, which the Binary column would read as
    # bloat and the .text column reads as what it is.
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
    # cost -- here, the Go runtime bringing up its scheduler and heap -- and it is
    # a result rather than overhead to be subtracted.
    output=$("${BINARY}" "$2" "$3" "$4")
    checksum=${output% *}
    elapsed_ns=${output#* }

    read_cpu_time
    printf '{"phase":"run","checksum":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s}\n' \
        "${checksum}" "${elapsed_ns}" "${user_usec}" "${system_usec}"
    ;;

disasm)
    # Evidence, not measurement -- and for this backend it is also the proof. `go
    # tool objdump` rather than binutils', because Go's assembler has its own
    # dialect and its own symbol names.
    #
    # Grep this listing for FMADD or FMSUB: on arm64 it must find nothing. If it
    # ever does, the rounding points in mandelbrot.go have been "cleaned up" and
    # the checksum is about to stop matching the rest of the table.
    listing=$(go tool objdump -s 'main.rowIterations' "${BINARY}")
    if ! printf '%s\n' "${listing}" | grep -qE '^[[:space:]]*mandelbrot\.go:'; then
        printf 'empty listing for main.rowIterations: the symbol is missing or is not code\n' >&2
        exit 1
    fi
    printf '%s\n' "${listing}"
    ;;

*)
    usage
    ;;
esac
