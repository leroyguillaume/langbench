//! Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
//! complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
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
//! invariant. See METHODOLOGY.md#the-strict-mode-invariant.
//!
//! Zig's float mode is `.strict` by default and this kernel leaves it there.
//! Relaxing it is `@setFloatMode(.optimized)` -- a statement *in the source*, not
//! a flag on the compiler -- so `fma` and `fast` cannot be build args over one
//! kernel the way they are for C. A kernel with a different float mode is a
//! different kernel, and this backend declares `strict` alone rather than pretend
//! otherwise. See `bench.yaml`.

const std = @import("std");

/// The viewport. Part of the cross-implementation contract: changing any of these
/// constants changes the reference checksum.
const X_MIN: f64 = -2.0;
const X_MAX: f64 = 0.5;
const Y_MIN: f64 = -1.25;
const Y_MAX: f64 = 1.25;

/// Sum the iteration counts of one row. The unit of work.
///
/// With any realistic n there are far more rows than threads, which is what the
/// dynamic hand-out below needs: the load is imbalanced by design (interior pixels
/// run to max_iter, exterior ones exit after a few iterations), so a static
/// contiguous split would measure the split rather than the backend.
fn rowIterations(row: u32, n: u32, max_iter: u32, dx: f64, dy: f64) u64 {
    const ci = Y_MIN + (@as(f64, @floatFromInt(row)) + 0.5) * dy;
    var sum: u64 = 0;

    var col: u32 = 0;
    while (col < n) : (col += 1) {
        const cr = X_MIN + (@as(f64, @floatFromInt(col)) + 0.5) * dx;
        var zr: f64 = 0.0;
        var zi: f64 = 0.0;
        var iter: u32 = 0;

        while (iter < max_iter) {
            const zr2 = zr * zr;
            const zi2 = zi * zi;
            if (zr2 + zi2 > 4.0) {
                break;
            }
            zi = 2.0 * zr * zi + ci;
            zr = zr2 - zi2 + cr;
            iter += 1;
        }
        sum += iter;
    }
    return sum;
}

/// One worker: takes the next row until there are none left, then parks its
/// subtotal where `main` can find it. No mutex -- the atomic cursor is the only
/// thing the workers share.
const Worker = struct {
    n: u32,
    max_iter: u32,
    dx: f64,
    dy: f64,
    next_row: *std.atomic.Value(u32),
    sum: u64 = 0,

    fn run(self: *Worker) void {
        var sum: u64 = 0;
        while (true) {
            const row = self.next_row.fetchAdd(1, .monotonic);
            if (row >= self.n) {
                break;
            }
            sum += rowIterations(row, self.n, self.max_iter, self.dx, self.dy);
        }
        self.sum = sum;
    }
};

fn parsePositive(text: []const u8, name: []const u8) u32 {
    const value = std.fmt.parseInt(u32, text, 10) catch {
        std.debug.print("{s} must be a positive integer, got `{s}`\n", .{ name, text });
        std.process.exit(2);
    };
    if (value == 0) {
        std.debug.print("{s} must be a positive integer, got `{s}`\n", .{ name, text });
        std.process.exit(2);
    }
    return value;
}

pub fn main() !void {
    const allocator = std.heap.smp_allocator;

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    if (args.len != 4) {
        std.debug.print("usage: {s} <n> <max_iter> <threads>\n", .{args[0]});
        std.process.exit(2);
    }

    // Never comptime constants: a backend would fold the whole computation away
    // and the benchmark would measure nothing, very quickly.
    const n = parsePositive(args[1], "n");
    const max_iter = parsePositive(args[2], "max_iter");
    const threads = parsePositive(args[3], "threads");

    const dx = (X_MAX - X_MIN) / @as(f64, @floatFromInt(n));
    const dy = (Y_MAX - Y_MIN) / @as(f64, @floatFromInt(n));

    var next_row = std.atomic.Value(u32).init(0);
    const workers = try allocator.alloc(Worker, threads);
    defer allocator.free(workers);
    const handles = try allocator.alloc(std.Thread, threads);
    defer allocator.free(handles);

    // Thread creation is inside the timer on purpose: spawning the pool is part
    // of what a parallel runtime costs, and the point is to compare runtimes.
    var timer = try std.time.Timer.start();

    for (workers, handles) |*worker, *handle| {
        worker.* = .{
            .n = n,
            .max_iter = max_iter,
            .dx = dx,
            .dy = dy,
            .next_row = &next_row,
        };
        handle.* = try std.Thread.spawn(.{}, Worker.run, .{worker});
    }
    for (handles) |handle| {
        handle.join();
    }
    const elapsed_ns = timer.read();

    // Summing 64-bit integers is associative, so the reduction order cannot
    // perturb the checksum however the threads happened to finish.
    var checksum: u64 = 0;
    for (workers) |worker| {
        checksum += worker.sum;
    }

    // Printing the checksum is what stops dead-code elimination from deleting the
    // loop above.
    var line: [64]u8 = undefined;
    const printed = try std.fmt.bufPrint(&line, "{d} {d}\n", .{ checksum, elapsed_ns });
    try std.fs.File.stdout().writeAll(printed);
}
