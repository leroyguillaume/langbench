# Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
# complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
#
# The program prints two integers on stdout: the checksum (the sum of every
# pixel's iteration count) and the wall-clock nanoseconds spent computing it.
#
# THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode it
# must be bit-identical to every other implementation's, C included. That holds
# because Julia's Float64 is an IEEE 754 double, because multiply, add, subtract
# and compare are correctly rounded, and because Julia never contracts a
# multiply-add into an FMA behind your back -- fusing is `muladd`, which you have
# to write. It also requires the arithmetic below to be evaluated in exactly the
# order the C kernel uses: do not "simplify" it.
# See METHODOLOGY.md#the-strict-mode-invariant.
#
# Julia compiles -- this kernel runs as native code, through LLVM, exactly like
# the C one -- but it compiles *during the run*, on first call, which is why the
# manifest declares an interpreter and no compiler: nothing is compiled ahead of
# the run. That JIT pause sits inside the timer, and it is the result this row
# exists to publish.
#
# No third-party dependency, by design: a package would drag in a load path and
# inflate the very startup this benchmark measures.

# The viewport. Part of the cross-implementation contract: changing any of these
# constants changes the reference checksum. `const`, because a non-constant global
# is boxed and type-unstable, and the hot loop would be reading `Any`.
const X_MIN = -2.0
const X_MAX = 0.5
const Y_MIN = -1.25
const Y_MAX = 1.25

"""
Sum the iteration counts of one row. The unit of work.

With any realistic `n` there are far more rows than threads, which is what the
dynamic hand-out below needs: the load is imbalanced by design (interior pixels
run to `max_iter`, exterior ones exit after a few iterations), so a static
contiguous split would measure the split rather than the backend.
"""
function row_iterations(row::Int, n::Int, max_iter::Int, dx::Float64, dy::Float64)::Int
    ci = Y_MIN + (row + 0.5) * dy
    total = 0

    for col in 0:(n - 1)
        cr = X_MIN + (col + 0.5) * dx
        zr = 0.0
        zi = 0.0
        iter = 0

        while iter < max_iter
            zr2 = zr * zr
            zi2 = zi * zi
            if zr2 + zi2 > 4.0
                break
            end
            zi = 2.0 * zr * zi + ci
            zr = zr2 - zi2 + cr
            iter += 1
        end
        total += iter
    end
    return total
end

"""
One worker's whole life: take the next row until there are none left.

`Threads.atomic_add!` returns the value *before* the addition, which is this
language's `fetch_add`: the cursor is the only state the workers share.
"""
function work(cursor::Threads.Atomic{Int}, n::Int, max_iter::Int, dx::Float64, dy::Float64)::Int
    total = 0
    while true
        row = Threads.atomic_add!(cursor, 1)
        if row >= n
            break
        end
        total += row_iterations(row, n, max_iter, dx, dy)
    end
    return total
end

function parse_positive(text::AbstractString, name::AbstractString)::Int
    value = tryparse(Int, text)
    if value === nothing || value <= 0
        println(stderr, "$name must be a positive integer, got `$text`")
        exit(2)
    end
    return value
end

function main()
    if length(ARGS) != 3
        println(stderr, "usage: mandelbrot.jl <n> <max_iter> <threads>")
        exit(2)
    end

    # Never module-level constants: a backend could fold the computation away.
    n = parse_positive(ARGS[1], "n")
    max_iter = parse_positive(ARGS[2], "max_iter")
    threads = parse_positive(ARGS[3], "threads")

    dx = (X_MAX - X_MIN) / n
    dy = (Y_MAX - Y_MIN) / n
    cursor = Threads.Atomic{Int}(0)

    # Spawning the tasks is inside the timer on purpose: what a parallel runtime
    # costs to start is part of what that runtime costs. So, here, is the JIT: the
    # first call to `row_iterations` compiles it, and that pause is a property of
    # this backend rather than an artefact to be warmed away.
    #
    # The kernel never asks how many threads the machine has: it spawns exactly as
    # many tasks as the harness asked for. `Threads.nthreads()` would answer with
    # whatever `--threads` the entrypoint passed, which is the same number by a
    # longer route -- and would quietly disagree with the harness the day a runtime
    # reads the cgroup quota differently.
    started = time_ns()
    tasks = [Threads.@spawn work(cursor, n, max_iter, dx, dy) for _ in 1:threads]
    # Summing 64-bit integers is associative, so the order in which the workers
    # finish cannot perturb the checksum.
    checksum = sum(fetch(task)::Int for task in tasks)
    elapsed_ns = time_ns() - started

    # Printing the checksum is what stops the compiler from eliding the loop above.
    println("$checksum $elapsed_ns")
    return 0
end

# Only when run as a program: the timed build phase `include`s this file to
# compile it, and must not execute the workload while doing so.
if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
