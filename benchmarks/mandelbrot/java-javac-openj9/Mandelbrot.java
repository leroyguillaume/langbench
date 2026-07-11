/*
 * Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
 * complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
 *
 * The program prints two integers on stdout: the checksum (the sum of every
 * pixel's iteration count) and the wall-clock nanoseconds spent computing it.
 *
 * THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode
 * it must be bit-identical across every compiler, every language and both ISAs.
 * Java makes that unusually easy to promise: since JEP 306 (Java 17) *all*
 * floating-point arithmetic is strictly IEEE 754 -- `strictfp` is the only
 * semantics there is -- and the JLS forbids the compiler *and* the JIT from
 * contracting `a * b + c` into an FMA. Fusing is `Math.fma`, which you have to
 * write. That is the opposite of Go, where the spec hands the compiler the same
 * freedom and gc takes it. See METHODOLOGY.md#the-languages-that-fuse-behind-your-back.
 *
 * It still requires the arithmetic below to be evaluated in exactly the order the
 * C kernel uses. Do not "simplify" it: any reassociation changes the last bit,
 * flips a boundary pixel from 999 to 1000 iterations, and breaks the invariant.
 *
 * Platform threads, not virtual ones: this workload is CPU-bound, and a virtual
 * thread would be a carrier thread wearing a hat. What is measured is the pool of
 * OS threads the JVM actually schedules.
 */

import java.util.concurrent.atomic.AtomicInteger;

public final class Mandelbrot {

    /* The viewport. Part of the cross-implementation contract: changing any of
     * these constants changes the reference checksum. */
    private static final double X_MIN = -2.0;
    private static final double X_MAX = 0.5;
    private static final double Y_MIN = -1.25;
    private static final double Y_MAX = 1.25;

    private Mandelbrot() {
    }

    /*
     * Sum the iteration counts of one row. The unit of work.
     *
     * With any realistic n there are far more rows than threads, which is what the
     * dynamic hand-out below needs: the load is imbalanced by design (interior
     * pixels run to maxIter, exterior ones exit after a few iterations), so a
     * static contiguous split would measure the split rather than the backend.
     */
    private static long rowIterations(int row, int n, int maxIter, double dx, double dy) {
        final double ci = Y_MIN + (row + 0.5) * dy;
        long sum = 0;

        for (int col = 0; col < n; ++col) {
            final double cr = X_MIN + (col + 0.5) * dx;
            double zr = 0.0;
            double zi = 0.0;
            int iter = 0;

            while (iter < maxIter) {
                final double zr2 = zr * zr;
                final double zi2 = zi * zi;
                if (zr2 + zi2 > 4.0) {
                    break;
                }
                zi = 2.0 * zr * zi + ci;
                zr = zr2 - zi2 + cr;
                ++iter;
            }
            sum += iter;
        }
        return sum;
    }

    private static int parsePositive(String text, String name) {
        try {
            final int value = Integer.parseInt(text);
            if (value > 0) {
                return value;
            }
        } catch (NumberFormatException ignored) {
            /* Falls through: an unparseable argument and a negative one are the
             * same mistake to whoever typed it. */
        }
        System.err.printf("%s must be a positive integer, got `%s`%n", name, text);
        System.exit(2);
        throw new AssertionError("unreachable");
    }

    public static void main(String[] args) throws InterruptedException {
        if (args.length != 3) {
            System.err.println("usage: Mandelbrot <n> <max_iter> <threads>");
            System.exit(2);
        }

        /* Never compile-time constants: a backend would fold the whole computation
         * away and the benchmark would measure nothing, very quickly. */
        final int n = parsePositive(args[0], "n");
        final int maxIter = parsePositive(args[1], "max_iter");
        final int threads = parsePositive(args[2], "threads");

        final double dx = (X_MAX - X_MIN) / n;
        final double dy = (Y_MAX - Y_MIN) / n;

        final AtomicInteger nextRow = new AtomicInteger(0);
        final long[] sums = new long[threads];
        final Thread[] pool = new Thread[threads];

        /* Thread creation is inside the timer on purpose: spawning the pool is part
         * of what a parallel runtime costs, and the point is to compare runtimes.
         * So is the JIT: the hot loop starts in the interpreter and is compiled by
         * C2 while it runs, and that pause is a property of this backend. */
        final long started = System.nanoTime();

        for (int i = 0; i < threads; ++i) {
            final int worker = i;
            pool[i] = new Thread(() -> {
                long sum = 0;
                for (;;) {
                    /* getAndIncrement returns the value *before* the addition: the
                     * same contract as C's atomic_fetch_add. */
                    final int row = nextRow.getAndIncrement();
                    if (row >= n) {
                        break;
                    }
                    sum += rowIterations(row, n, maxIter, dx, dy);
                }
                sums[worker] = sum;
            });
            pool[i].start();
        }
        for (final Thread worker : pool) {
            worker.join();
        }
        final long elapsedNs = System.nanoTime() - started;

        /* Summing 64-bit integers is associative, so the reduction order cannot
         * perturb the checksum however the threads happened to finish. */
        long checksum = 0;
        for (final long sum : sums) {
            checksum += sum;
        }

        /* Printing the checksum is what stops the JIT from eliding the loop above. */
        System.out.println(checksum + " " + elapsedNs);
    }
}
