"""Mandelbrot, CPython, `multiprocessing`.

Each pixel of an N x N grid maps onto a fixed viewport of the complex plane and
iterates ``z <- z^2 + c`` until ``|z| > 2`` or ``max_iter`` is reached. The
program prints two integers on stdout: the checksum (the sum of every pixel's
iteration count) and the wall-clock nanoseconds spent computing it.

THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode it
must be bit-identical to every other implementation's, C included. That holds
because CPython's floats are IEEE 754 doubles, because multiply, add, subtract
and compare are correctly rounded, and because the interpreter never contracts a
multiply-add into an FMA. It also requires the arithmetic below to be evaluated
in exactly the order the C kernel uses -- do not "simplify" it.
See METHODOLOGY.md#the-strict-mode-invariant.

Processes, not threads: the GIL serialises CPU-bound bytecode, so `threading`
would measure the GIL rather than the machine. `multiprocessing` is what CPython
actually offers for this workload, and the cost of forking the pool is part of
what that choice costs.

No third-party dependency, by design: `typer` or `numpy` would inflate the very
startup and build times this benchmark measures.
"""

from __future__ import annotations

import multiprocessing
import sys
import time
from dataclasses import dataclass

# The viewport. Part of the cross-implementation contract: changing any of these
# constants changes the reference checksum.
X_MIN = -2.0
X_MAX = 0.5
Y_MIN = -1.25
Y_MAX = 1.25


@dataclass(frozen=True, slots=True)
class Grid:
    """What a worker needs to know. Inherited by `fork`, never pickled."""

    n: int
    max_iter: int
    dx: float
    dy: float


_GRID: Grid | None = None


def _init_worker(grid: Grid) -> None:
    global _GRID  # noqa: PLW0603 -- the pool initializer's whole purpose
    _GRID = grid


def _row_iterations(row: int) -> int:
    """Sum the iteration counts of one row. The unit of work."""
    if _GRID is None:
        raise RuntimeError("worker started without an initializer")
    grid = _GRID

    ci = Y_MIN + (row + 0.5) * grid.dy
    total = 0
    for col in range(grid.n):
        cr = X_MIN + (col + 0.5) * grid.dx
        zr = 0.0
        zi = 0.0
        iterations = 0
        while iterations < grid.max_iter:
            zr2 = zr * zr
            zi2 = zi * zi
            if zr2 + zi2 > 4.0:
                break
            zi = 2.0 * zr * zi + ci
            zr = zr2 - zi2 + cr
            iterations += 1
        total += iterations
    return total


def _parse_positive(text: str, name: str) -> int:
    try:
        value = int(text)
    except ValueError:
        value = 0
    if value <= 0:
        print(f"{name} must be a positive integer, got `{text}`", file=sys.stderr)
        raise SystemExit(2)
    return value


def main() -> int:
    if len(sys.argv) != 4:
        print(f"usage: {sys.argv[0]} <n> <max_iter> <threads>", file=sys.stderr)
        return 2

    # Never module-level constants: a backend could fold the computation away.
    n = _parse_positive(sys.argv[1], "n")
    max_iter = _parse_positive(sys.argv[2], "max_iter")
    threads = _parse_positive(sys.argv[3], "threads")

    grid = Grid(n=n, max_iter=max_iter, dx=(X_MAX - X_MIN) / n, dy=(Y_MAX - Y_MIN) / n)

    # The load is imbalanced by design, so chunks are handed out on demand and
    # there are at least `4 * threads` of them. A static split would measure the
    # split rather than the backend.
    chunksize = max(1, n // (4 * threads))

    # `fork` explicitly: CPython 3.14 changes the Linux default to `forkserver`,
    # and the start method is part of what we are measuring.
    context = multiprocessing.get_context("fork")

    # Forking the pool is inside the timer on purpose: spawning workers is part
    # of what a parallel runtime costs, and the point is to compare runtimes.
    started = time.perf_counter_ns()
    with context.Pool(
        processes=threads, initializer=_init_worker, initargs=(grid,)
    ) as pool:
        # Summing 64-bit integers is associative, so the order in which the
        # workers finish cannot perturb the checksum.
        checksum = sum(
            pool.imap_unordered(_row_iterations, range(n), chunksize=chunksize)
        )
    elapsed_ns = time.perf_counter_ns() - started

    # Printing the checksum is what stops a backend from eliding the loop above.
    # This is the program's result, not a diagnostic.
    print(f"{checksum} {elapsed_ns}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
