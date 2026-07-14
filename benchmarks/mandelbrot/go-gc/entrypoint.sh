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

# The ISA target, translated out of gcc's spelling into Go's -- and Go is the backend
# where the translation is lossy, which is the whole reason a sample carries `isa`
# beside its `mode`.
#
# Go takes no `-march` and knows no `native`. It takes a per-architecture environment
# variable naming a microarchitecture *level* (GOAMD64=v1..v4, GOARM64=v8.0..v9.5),
# it never asks the CPU which level it is on, and it silently ignores the variable
# that does not belong to the architecture it is building for -- so `GOAMD64=v3` on
# arm64 is not an error, it is a no-op, and the row would claim a target it never had.
# Anything unknown therefore dies here, and `native` is resolved against TARGETARCH,
# which the Dockerfile pins into the image from BuildKit.
#
# What `native` means here, and it is the same rule on both architectures: **the highest
# level this CPU is known to have**, never the highest level Go can name. That is what
# `-march=native` *is* -- gcc inspects the machine it is standing on -- and since Go
# refuses to do the inspection, this entrypoint does it. Translating a flag is the job;
# the fact that Go's half of the translation is missing does not make it optional.
#
#   x86-64: the psABI levels are feature sets, so the CPU is asked which one it clears.
#           Naming v4 unconditionally -- the highest Go *can* say -- would be a claim and
#           not a measurement: v4 is AVX-512, the campaign's own baseline is v3 (AVX2),
#           and the Go runtime refuses to start a v4 binary on a CPU without it ("This
#           program can only be run on AMD64 processors with v4 microarchitecture
#           support", runtime/asm_amd64.s). On every AVX2 bench machine that is not a
#           quarantined unit worth having -- it is the `native` row deleting itself, and
#           a campaign that cannot say what native buys Go.
#
#           Detected, the answer on such a machine is v3: the same level as the baseline,
#           the same binary, and the two rows tie. That is not a missing result. It is
#           the result -- Go has no instruction set left to reach for here.
#
#   arm64:  GOARM64=v8.2 -- the baseline's own value, and yes, that means the two modes
#           build the same binary here. Go 1.25 will accept up to v9.5, but GOARM64
#           names an ARM *architecture version*, nothing verifies it against the CPU at
#           startup, and no machine that will ever run this campaign is an ARMv9.5. It
#           would be a claim rather than a measurement -- and it would buy nothing:
#           `go tool objdump` of main.rowIterations is byte for byte identical at v8.2
#           and at v9.5. So `native` takes the highest level the machine is *known* to
#           have, `isa` says v8.2 in both modes, and the rows tie because they are one
#           program.
#
# `isa` is what Go actually set, never what the harness asked for: Go cannot say
# `native`, so this row never claims it. It is a JSON value, not a string -- an absence
# is `null`, never the empty string.

# The highest GOAMD64 level this CPU clears, from the psABI's own feature sets. This is
# the CPU detection `-march=native` performs for gcc and Go declines to perform for
# itself; doing it here is translating the flag, not inventing a policy.
#
# A level is claimed only when *every* feature it requires is present -- v4 needs the
# five AVX-512 subsets, not merely `avx512f` -- because a partial match is how a binary
# ends up targeting an instruction the CPU does not have.
detect_goamd64() {
    flags=$(awk '/^flags/ { $1=""; $2=""; print; exit }' /proc/cpuinfo)
    has() {
        case " ${flags} " in
        *" $1 "*) return 0 ;;
        *) return 1 ;;
        esac
    }
    if has avx512f && has avx512bw && has avx512cd && has avx512dq && has avx512vl; then
        printf 'v4\n'
    elif has avx && has avx2 && has bmi1 && has bmi2 && has fma && has movbe; then
        printf 'v3\n'
    elif has sse4_2 && has popcnt; then
        printf 'v2\n'
    else
        printf 'v1\n'
    fi
}

export_march() {
    case "${MARCH:-}" in
    '')
        # Nothing pinned: Go's own defaults apply (v1 / v8.0), and this row claims no
        # ISA it did not choose.
        isa=null
        ;;
    x86-64-v3)
        GOAMD64=v3
        export GOAMD64
        isa='"v3"'
        ;;
    armv8.2-a)
        GOARM64=v8.2
        export GOARM64
        isa='"v8.2"'
        ;;
    native)
        case "${TARGETARCH:-}" in
        amd64)
            GOAMD64=$(detect_goamd64)
            export GOAMD64
            isa="\"${GOAMD64}\""
            ;;
        arm64)
            GOARM64=v8.2
            export GOARM64
            isa='"v8.2"'
            ;;
        *)
            printf 'MARCH=native needs TARGETARCH: %s does not say whether to set GOAMD64 or\n' "${TARGETARCH:-<unset>}" >&2
            printf 'GOARM64, and go ignores the wrong one in silence. BuildKit sets it at build.\n' >&2
            exit 1
            ;;
        esac
        ;;
    *)
        printf 'unknown MARCH for go: %s. Add its GOAMD64/GOARM64 spelling to export_march()\n' "${MARCH}" >&2
        printf 'rather than letting go build for a level nobody asked for.\n' >&2
        exit 1
        ;;
    esac
}

# Single source of truth for the compiler flags, shared by the image build and by
# the timed rebuild.
#
# There is no floating-point flag here, and Go is the one language in this table where
# that is a statement about the *source* rather than about the compiler. The spec lets
# gc fuse a multiply-add whenever it likes, and on arm64 it takes the offer; the kernel
# forbids it with explicit float64() rounding points, because the checksum is not
# negotiable. A fused build would be a different program -- not a flag away -- so
# strict floating point survives every ISA target this row is given, and no mode can
# ask for anything else. Read the header of mandelbrot.go: the unrounded version really
# does return a different checksum, on this very machine.
compile_to() {
    output=$1
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
# Resolved once, up front, for every phase: the compiling phases need the exported
# GOAMD64/GOARM64, and the measured phases need `isa` to report. It costs no process
# -- a `uname` would be one, and one fork inside a measured phase lands in the same
# cgroup the CPU column is read from.
export_march

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
    #
    # It refills std at the *campaign's* ISA level, which is why export_march ran
    # first: a std compiled at another level is a cache that will be missed.
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
    read_peak_memory
    printf '{"phase":"build","elapsed_ns":%s,"isa":%s,"user_usec":%s,"system_usec":%s,"binary_bytes":%s,"binary_stripped_bytes":%s,"text_bytes":%s,"peak_bytes":%s}\n' \
        "${elapsed_ns}" "${isa}" "${user_usec}" "${system_usec}" \
        "${binary_bytes}" "${binary_stripped_bytes}" "${text_bytes}" "${peak_bytes}"
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
    read_peak_memory
    printf '{"phase":"run","checksum":%s,"isa":%s,"elapsed_ns":%s,"user_usec":%s,"system_usec":%s,"peak_bytes":%s}\n' \
        "${checksum}" "${isa}" "${elapsed_ns}" "${user_usec}" "${system_usec}" "${peak_bytes}"
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
