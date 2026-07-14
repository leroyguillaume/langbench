# Mandelbrot, Go, gc.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image.
#
# THE GO TRAP, and the two lines at the bottom of this file that avoid it.
# Building the binary during `docker build` does not only produce the binary -- it
# also fills GOCACHE with the compiled form of *our package*. Ship that image and
# the timed rebuild finds its answer already in the cache, returns in a few
# milliseconds, and Go tops the Build column by not building anything.
#
# So the cache is wiped and then refilled with the standard library alone:
# hot for the toolchain, cold for our code. That is the same rule that says to warm
# Rust's precompiled std and Deno's typings -- Go is only the loudest case, because
# it compiles its own std from source on a cold cache, and without the refill this
# column would read "Go compiles half of net/http" instead of "Go compiles one
# file". See site/src/content/methodology.md#how-a-run-is-measured.
#
# GOCACHE lives in the image, not in the tmpfs, and that is what makes it hold:
# every `docker run` starts from the image layers with a fresh writable layer, so
# each measured build begins from exactly this state -- std warm, our package
# absent -- no matter how many rounds the campaign runs.
FROM golang:1.25-trixie@sha256:75eb6e343bdac854462811220a5bcae3442af3fb25e8c8644c7494e5e5545aed

# Floating-point semantics, ISA baseline and job count arrive as build args, so
# one Dockerfile covers every mode the backend distinguishes -- which, here, is
# `strict` alone. See `bench.yaml`.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS=1

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="1.25" \
      langbench.flags="march=${MARCH} fp=${FP_MODE}"

# GO111MODULE=off: one file, no dependency, no go.mod. GOPROXY=off and GOFLAGS
# belt-and-braces with `--network=none` -- a build that tries to fetch must fail
# loudly, not add four seconds to the measurement.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    GO111MODULE=off \
    GOPROXY=off \
    GOCACHE=/usr/local/share/go-cache \
    GOPATH=/usr/local/share/go \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.go .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The trap, sprung deliberately and then disarmed. `go clean -cache` throws away
# everything the install step cached -- our package *and* std -- and `go build std`
# puts std back. What is left is a cache that can compile our one file quickly and
# has never seen it.
RUN /usr/local/bin/entrypoint.sh warm-cache

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot \
 && chown -R mandelbrot:mandelbrot "${GOCACHE}"

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
