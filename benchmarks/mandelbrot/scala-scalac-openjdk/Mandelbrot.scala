// Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
// complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
//
// The program prints two integers on stdout: the checksum (the sum of every
// pixel's iteration count) and the wall-clock nanoseconds spent computing it.
//
// THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode it
// must be bit-identical to every other implementation's, C included. Scala on the
// JVM inherits Java's arithmetic: a Double is an IEEE 754 double, every operation
// is strictly rounded, and neither scalac nor HotSpot's JIT may contract
// `a * b + c` into an FMA. It still requires the arithmetic below to be evaluated
// in exactly the order the C kernel uses: do not "simplify" it.
// See METHODOLOGY.md#the-strict-mode-invariant.
//
// A `while` loop in a language built for `foldLeft`, and that is deliberate. This
// benchmark measures backends, not styles: the C kernel's loop is the contract, and
// every implementation runs the same arithmetic in the same order. A version
// written over a lazy `Iterator` with a `foldLeft` would be a fine piece of Scala
// and a different program -- it would measure the collections library, and the row
// would quietly stop being comparable to the other sixteen. The idiomatic thing to
// do in a cross-language benchmark is to write the same algorithm.
//
// java.util.concurrent, not cats-effect or Akka: no third-party dependency, and a
// green-thread scheduler would be a different experiment. Threads are what the JVM
// actually schedules. (Scala Native reimplements both, so this same file compiles
// there too, unchanged -- which is what makes scala-scala-native a backend swap
// rather than a rewrite.)
//
// HOW THE ESCAPE IS SPELLED. Scala has no `break` statement, and the obvious
// workaround -- an `escaped` flag in the loop guard -- puts a boolean test in front
// of every iteration of the hot loop (the checksum is the count of exactly those
// iterations: ~1.0e9 at the declared params), which is the workaround showing up in
// the measurement rather than the backend. The escape is therefore a
// `return` out of `pixelIterations`: a *local* return, because the loop it leaves is
// in the method that owns it, so scalac emits an ordinary IRETURN -- no flag, no
// `scala.util.boundary`, no exception, and no closure for a JIT to see through. Same
// arithmetic, same order, same checksum, and the same shape of inner loop the C,
// Java and Kotlin rows run, which is what makes those comparisons about the backend.

import java.util.concurrent.atomic.AtomicInteger

object Mandelbrot:

  // The viewport. Part of the cross-implementation contract: changing any of these
  // constants changes the reference checksum.
  private val XMin = -2.0
  private val XMax = 0.5
  private val YMin = -1.25
  private val YMax = 1.25

  /** Iterations one pixel completes before it escapes, capped at `maxIter`.
    *
    * The escape is a `return`, which is what gives this row the same inner loop as
    * the C kernel's `break` -- see the note at the top of the file. `iter` counts
    * *completed* iterations: a pixel that escapes on its k-th test contributes k,
    * not `maxIter`.
    */
  private def pixelIterations(cr: Double, ci: Double, maxIter: Int): Int =
    var zr = 0.0
    var zi = 0.0
    var iter = 0

    while iter < maxIter do
      val zr2 = zr * zr
      val zi2 = zi * zi
      if zr2 + zi2 > 4.0 then return iter
      zi = 2.0 * zr * zi + ci
      zr = zr2 - zi2 + cr
      iter += 1

    iter

  /** Sum the iteration counts of one row. The unit of work.
    *
    * With any realistic `n` there are far more rows than threads, which is what the
    * dynamic hand-out below needs: the load is imbalanced by design (interior
    * pixels run to `maxIter`, exterior ones exit after a few iterations), so a
    * static contiguous split would measure the split rather than the backend.
    */
  private def rowIterations(row: Int, n: Int, maxIter: Int, dx: Double, dy: Double): Long =
    val ci = YMin + (row + 0.5) * dy
    var sum = 0L
    var col = 0

    while col < n do
      sum += pixelIterations(XMin + (col + 0.5) * dx, ci, maxIter)
      col += 1

    sum

  private def parsePositive(text: String, name: String): Int =
    text.toIntOption match
      case Some(value) if value > 0 => value
      case _ =>
        System.err.println(s"$name must be a positive integer, got `$text`")
        System.exit(2)
        throw AssertionError("unreachable")

  def main(args: Array[String]): Unit =
    if args.length != 3 then
      System.err.println("usage: Mandelbrot <n> <max_iter> <threads>")
      System.exit(2)

    // Never top-level constants: a backend could fold the computation away.
    val n = parsePositive(args(0), "n")
    val maxIter = parsePositive(args(1), "max_iter")
    val threads = parsePositive(args(2), "threads")

    val dx = (XMax - XMin) / n
    val dy = (YMax - YMin) / n

    val nextRow = AtomicInteger(0)
    val sums = Array.fill(threads)(0L)

    // Thread creation is inside the timer on purpose: spawning the pool is part of
    // what a parallel runtime costs. So is the JIT: the hot loop starts in the
    // interpreter and C2 compiles it while it runs, and that pause is a property of
    // this backend rather than an artefact to be warmed away.
    val started = System.nanoTime()

    val pool = (0 until threads).map { worker =>
      val thread = Thread: () =>
        var sum = 0L
        var running = true
        while running do
          // getAndIncrement returns the value *before* the addition: the same
          // contract as C's atomic_fetch_add.
          val row = nextRow.getAndIncrement()
          if row >= n then running = false
          else sum += rowIterations(row, n, maxIter, dx, dy)
        sums(worker) = sum
      thread.start()
      thread
    }
    pool.foreach(_.join())
    val elapsedNs = System.nanoTime() - started

    // Summing 64-bit integers is associative, so the order in which the threads
    // finish cannot perturb the checksum.
    val checksum = sums.sum

    // Printing the checksum is what stops the JIT from eliding the loop above.
    println(s"$checksum $elapsedNs")
