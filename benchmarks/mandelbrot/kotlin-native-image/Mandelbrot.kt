// Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
// complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
//
// The program prints two integers on stdout: the checksum (the sum of every
// pixel's iteration count) and the wall-clock nanoseconds spent computing it.
//
// THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode it
// must be bit-identical to every other implementation's, C included. Kotlin/JVM
// inherits Java's arithmetic: a Double is an IEEE 754 double, every operation is
// strictly rounded, and neither kotlinc nor HotSpot's JIT may contract `a * b + c`
// into an FMA -- fusing is `Math.fma`, which you have to write. It still requires
// the arithmetic below to be evaluated in exactly the order the C kernel uses: do
// not "simplify" it. See METHODOLOGY.md#the-strict-mode-invariant.
//
// WHAT THIS ROW IS FOR. Kotlin and Java compile to the same bytecode, run on the
// same HotSpot, and JIT to the same machine code. So the honest expectation is
// that this row's compute time equals java-javac-openjdk's exactly, and the experiment is
// whether anything else moves -- the compiler (kotlinc is famously slower than
// javac), and the stdlib the runtime has to load. A row that confirms an
// expectation is still a row worth having: "Kotlin is slower than Java" is a claim
// people make.
//
// java.util.concurrent, not kotlinx.coroutines: a third-party dependency is
// forbidden here, and a coroutine dispatcher would be a different experiment
// anyway. Threads are what the JVM actually schedules.

import java.util.concurrent.atomic.AtomicInteger
import kotlin.system.exitProcess

// The viewport. Part of the cross-implementation contract: changing any of these
// constants changes the reference checksum.
private const val X_MIN = -2.0
private const val X_MAX = 0.5
private const val Y_MIN = -1.25
private const val Y_MAX = 1.25

/**
 * Sum the iteration counts of one row. The unit of work.
 *
 * With any realistic [n] there are far more rows than threads, which is what the
 * dynamic hand-out below needs: the load is imbalanced by design (interior pixels
 * run to [maxIter], exterior ones exit after a few iterations), so a static
 * contiguous split would measure the split rather than the backend.
 */
private fun rowIterations(row: Int, n: Int, maxIter: Int, dx: Double, dy: Double): Long {
    val ci = Y_MIN + (row + 0.5) * dy
    var sum = 0L

    for (col in 0 until n) {
        val cr = X_MIN + (col + 0.5) * dx
        var zr = 0.0
        var zi = 0.0
        var iter = 0

        while (iter < maxIter) {
            val zr2 = zr * zr
            val zi2 = zi * zi
            if (zr2 + zi2 > 4.0) {
                break
            }
            zi = 2.0 * zr * zi + ci
            zr = zr2 - zi2 + cr
            ++iter
        }
        sum += iter
    }
    return sum
}

private fun parsePositive(text: String, name: String): Int {
    val value = text.toIntOrNull()
    if (value == null || value <= 0) {
        System.err.println("$name must be a positive integer, got `$text`")
        exitProcess(2)
    }
    return value
}

fun main(args: Array<String>) {
    if (args.size != 3) {
        System.err.println("usage: Mandelbrot <n> <max_iter> <threads>")
        exitProcess(2)
    }

    // Never top-level constants: a backend could fold the computation away.
    val n = parsePositive(args[0], "n")
    val maxIter = parsePositive(args[1], "max_iter")
    val threads = parsePositive(args[2], "threads")

    val dx = (X_MAX - X_MIN) / n
    val dy = (Y_MAX - Y_MIN) / n

    val nextRow = AtomicInteger(0)
    val sums = LongArray(threads)

    // Thread creation is inside the timer on purpose: spawning the pool is part of
    // what a parallel runtime costs. So is the JIT: the hot loop starts in the
    // interpreter and C2 compiles it while it runs, and that pause is a property of
    // this backend rather than an artefact to be warmed away.
    val started = System.nanoTime()

    val pool = (0 until threads).map { worker ->
        Thread {
            var sum = 0L
            while (true) {
                // getAndIncrement returns the value *before* the addition: the same
                // contract as C's atomic_fetch_add.
                val row = nextRow.getAndIncrement()
                if (row >= n) {
                    break
                }
                sum += rowIterations(row, n, maxIter, dx, dy)
            }
            sums[worker] = sum
        }.apply { start() }
    }
    pool.forEach { it.join() }
    val elapsedNs = System.nanoTime() - started

    // Summing 64-bit integers is associative, so the order in which the threads
    // finish cannot perturb the checksum.
    val checksum = sums.sum()

    // Printing the checksum is what stops the JIT from eliding the loop above.
    println("$checksum $elapsedNs")
}
