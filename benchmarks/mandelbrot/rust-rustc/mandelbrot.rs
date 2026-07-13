//! Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
//! complex plane and iterates `z <- z^2 + c` until `|z| > 2` or `max_iter` is
//! reached.
//!
//! The program prints two integers on stdout: the checksum (the sum of every
//! pixel's iteration count) and the wall-clock nanoseconds spent computing it.
//!
//! THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode
//! it must be bit-identical across every compiler, every language and both ISAs.
//! That only holds because the operations below are multiply, add, subtract and
//! compare, all correctly rounded by IEEE 754 -- and because every implementation
//! evaluates them in exactly this order. Do not "simplify" the arithmetic: any
//! reassociation, or an FMA contraction of `zr2 - zi2 + cr`, changes the last
//! bit, flips a boundary pixel from 999 to 1000 iterations, and breaks the
//! invariant. Rust makes this easy to keep: `f64` arithmetic is IEEE 754 and the
//! compiler is never allowed to contract or reassociate it -- there is no
//! `-ffast-math` to reach for, which is why this backend declares `strict` alone.
//! See site/src/content/methodology/floating-point.md#the-strict-mode-invariant.
//!
//! `std::thread` and an atomic cursor, never `rayon`: a third-party work-stealing
//! scheduler is a different experiment, and this one is about the backend.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

/// The viewport. Part of the cross-implementation contract: changing any of these
/// constants changes the reference checksum.
const X_MIN: f64 = -2.0;
const X_MAX: f64 = 0.5;
const Y_MIN: f64 = -1.25;
const Y_MAX: f64 = 1.25;

/// What every worker needs to know, and nothing it may change.
struct Grid {
    n: u32,
    max_iter: u32,
    dx: f64,
    dy: f64,
}

/// Sum the iteration counts of one row. The unit of work.
///
/// With any realistic `n` there are far more rows than threads, which is what the
/// dynamic hand-out below needs: the load is imbalanced by design (interior pixels
/// run to `max_iter`, exterior ones exit after a few iterations), so a static
/// contiguous split would measure the split rather than the backend.
fn row_iterations(row: u32, grid: &Grid) -> u64 {
    let ci = Y_MIN + (f64::from(row) + 0.5) * grid.dy;
    let mut sum: u64 = 0;

    for col in 0..grid.n {
        let cr = X_MIN + (f64::from(col) + 0.5) * grid.dx;
        let mut zr = 0.0_f64;
        let mut zi = 0.0_f64;
        let mut iter: u32 = 0;

        while iter < grid.max_iter {
            let zr2 = zr * zr;
            let zi2 = zi * zi;
            if zr2 + zi2 > 4.0 {
                break;
            }
            zi = 2.0 * zr * zi + ci;
            zr = zr2 - zi2 + cr;
            iter += 1;
        }
        sum += u64::from(iter);
    }
    sum
}

fn parse_positive(text: &str, name: &str) -> u32 {
    match text.parse::<u32>() {
        Ok(value) if value > 0 => value,
        _ => {
            eprintln!("{name} must be a positive integer, got `{text}`");
            std::process::exit(2);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: {} <n> <max_iter> <threads>", args[0]);
        std::process::exit(2);
    }

    // Never compile-time constants: a backend would fold the whole computation
    // away and the benchmark would measure nothing, very quickly.
    let n = parse_positive(&args[1], "n");
    let max_iter = parse_positive(&args[2], "max_iter");
    let threads = parse_positive(&args[3], "threads");

    let grid = Grid {
        n,
        max_iter,
        dx: (X_MAX - X_MIN) / f64::from(n),
        dy: (Y_MAX - Y_MIN) / f64::from(n),
    };
    let next_row = AtomicU32::new(0);

    // Thread creation is inside the timer on purpose: spawning the pool is part
    // of what a parallel runtime costs, and the point is to compare runtimes.
    let started = Instant::now();

    // Scoped threads: the workers borrow `grid` and `next_row` off the stack, so
    // there is no `Arc` to allocate and no reference count to touch. The scope is
    // what proves to the compiler that the borrow outlives the thread.
    let sums: Vec<u64> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut sum: u64 = 0;
                    loop {
                        let row = next_row.fetch_add(1, Ordering::Relaxed);
                        if row >= grid.n {
                            break;
                        }
                        sum += row_iterations(row, &grid);
                    }
                    sum
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a worker panicked"))
            .collect()
    });
    let elapsed = started.elapsed();

    // Summing 64-bit integers is associative, so the reduction order cannot
    // perturb the checksum however the threads happened to finish.
    let checksum: u64 = sums.iter().sum();

    // Printing the checksum is what stops dead-code elimination from deleting the
    // loop above.
    println!("{checksum} {}", elapsed.as_nanos());
}
