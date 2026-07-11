/*
 * Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
 * complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
 *
 * The program prints two integers on stdout: the checksum (the sum of every
 * pixel's iteration count) and the wall-clock nanoseconds spent computing it.
 *
 * THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode
 * it must be bit-identical across every compiler, every language and both ISAs.
 * That only holds because the operations below are multiply, add, subtract and
 * compare, all correctly rounded by IEEE 754 -- and because every implementation
 * evaluates them in exactly this order. Do not "simplify" the arithmetic: any
 * reassociation, or an FMA contraction of `zr2 - zi2 + cr`, changes the last bit,
 * flips a boundary pixel from 999 to 1000 iterations, and breaks the invariant.
 * See METHODOLOGY.md#the-strict-mode-invariant.
 *
 * The kernel is the C one, rewritten in the idioms a C++ programmer would
 * actually reach for -- `std::thread`, `std::atomic`, `std::vector` -- and not a
 * `.cpp` extension bolted onto C. That is the point of the row: the same
 * arithmetic through the language's own abstractions, to find out what those
 * abstractions cost. (Nothing, at -O3. That is the result.)
 */

#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>
#include <vector>

namespace {

/* The viewport. Part of the cross-implementation contract: changing any of
 * these constants changes the reference checksum. */
constexpr double X_MIN = -2.0;
constexpr double X_MAX = 0.5;
constexpr double Y_MIN = -1.25;
constexpr double Y_MAX = 1.25;

/* The unit of work is one row. With any realistic N there are far more rows
 * than threads, which is what the dynamic hand-out below needs: the load is
 * imbalanced by design (interior pixels run to max_iter, exterior ones exit
 * after a few iterations), so a static contiguous split would measure the split
 * rather than the backend. */
std::uint64_t row_iterations(std::uint32_t row, std::uint32_t n, std::uint32_t max_iter, double dx,
                             double dy)
{
    const double ci = Y_MIN + (static_cast<double>(row) + 0.5) * dy;
    std::uint64_t sum = 0;

    for (std::uint32_t col = 0; col < n; ++col) {
        const double cr = X_MIN + (static_cast<double>(col) + 0.5) * dx;
        double zr = 0.0;
        double zi = 0.0;
        std::uint32_t iter = 0;

        while (iter < max_iter) {
            const double zr2 = zr * zr;
            const double zi2 = zi * zi;
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

std::uint32_t parse_positive(const char *text, const char *name)
{
    try {
        std::size_t consumed = 0;
        const unsigned long value = std::stoul(text, &consumed);
        if (consumed == std::string(text).size() && value != 0 && value <= UINT32_MAX) {
            return static_cast<std::uint32_t>(value);
        }
    } catch (const std::exception &) {
        /* Falls through to the diagnostic below: an unparseable argument and an
         * out-of-range one are the same mistake to whoever typed it. */
    }
    std::fprintf(stderr, "%s must be a positive integer, got `%s`\n", name, text);
    std::exit(2);
}

}  // namespace

int main(int argc, char **argv)
{
    if (argc != 4) {
        std::fprintf(stderr, "usage: %s <n> <max_iter> <threads>\n", argv[0]);
        return 2;
    }

    /* Never compile-time constants: a backend would fold the whole computation
     * away and the benchmark would measure nothing, very quickly. */
    const std::uint32_t n = parse_positive(argv[1], "n");
    const std::uint32_t max_iter = parse_positive(argv[2], "max_iter");
    const std::uint32_t threads = parse_positive(argv[3], "threads");

    const double dx = (X_MAX - X_MIN) / static_cast<double>(n);
    const double dy = (Y_MAX - Y_MIN) / static_cast<double>(n);

    std::atomic<std::uint32_t> next_row{0};
    std::vector<std::uint64_t> sums(threads, 0);
    std::vector<std::thread> pool;
    pool.reserve(threads);

    /* Thread creation is inside the timer on purpose: spawning the pool is part
     * of what a parallel runtime costs, and the point is to compare runtimes. */
    const auto started = std::chrono::steady_clock::now();

    for (std::uint32_t i = 0; i < threads; ++i) {
        pool.emplace_back([&, i] {
            std::uint64_t sum = 0;
            for (;;) {
                const std::uint32_t row = next_row.fetch_add(1, std::memory_order_relaxed);
                if (row >= n) {
                    break;
                }
                sum += row_iterations(row, n, max_iter, dx, dy);
            }
            sums[i] = sum;
        });
    }
    for (std::thread &worker : pool) {
        worker.join();
    }
    const auto finished = std::chrono::steady_clock::now();

    /* Summing 64-bit integers is associative, so the reduction order cannot
     * perturb the checksum however the threads happened to finish. */
    std::uint64_t checksum = 0;
    for (const std::uint64_t sum : sums) {
        checksum += sum;
    }

    const auto elapsed_ns =
        std::chrono::duration_cast<std::chrono::nanoseconds>(finished - started).count();

    /* Printing the checksum is what stops dead-code elimination from deleting
     * the loop above. */
    std::printf("%llu %lld\n", static_cast<unsigned long long>(checksum),
                static_cast<long long>(elapsed_ns));
    return 0;
}
