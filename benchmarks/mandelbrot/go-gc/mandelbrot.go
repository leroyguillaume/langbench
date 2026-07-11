// Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
// complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
//
// The program prints two integers on stdout: the checksum (the sum of every
// pixel's iteration count) and the wall-clock nanoseconds spent computing it.
//
// THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode
// it must be bit-identical across every compiler, every language and both ISAs.
// See METHODOLOGY.md#the-strict-mode-invariant.
//
// AND GO IS THE LANGUAGE THAT WILL BREAK IT IF YOU LET IT. Read this before you
// "clean up" the arithmetic below.
//
// The Go specification explicitly permits an implementation to "combine multiple
// floating-point operations into a single fused operation, possibly across
// statements, and produce a result that differs from the value obtained by
// executing and rounding the instructions individually". On arm64 the gc compiler
// takes that permission: it fuses `2.0*zr*zi + ci` into an FMADD, and -- less
// obviously -- it fuses `zr2 - zi2` back into an FMSUB on `zr*zr`, because it can
// see where zr2 came from. Written the natural way, this kernel returns 33209560
// where every other language in the table returns 33209574. It is not slower or
// buggier; it is computing something else, and only the checksum would have told
// us.
//
// The spec also gives the one escape: "An explicit floating-point type conversion
// rounds to the precision of the target type, preventing fusion that would
// discard that rounding." That is what every `float64(...)` below is for. They are
// not casts -- the operands are already float64 -- they are rounding points, and
// they are the only way the language lets you say "round here". Delete one and the
// campaign fails on the checksum, which is exactly what that gate is for.
//
// C says the same thing with `-ffp-contract=off`, on the command line, where you
// can see it. Go says it in the source, which is why this kernel has no `fma`
// mode: a fused build would be a different program. See bench.yaml.
package main

import (
	"fmt"
	"os"
	"runtime"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

// The viewport. Part of the cross-implementation contract: changing any of these
// constants changes the reference checksum.
const (
	xMin = -2.0
	xMax = 0.5
	yMin = -1.25
	yMax = 1.25
)

// rowIterations sums the iteration counts of one row. The unit of work.
//
// With any realistic n there are far more rows than threads, which is what the
// dynamic hand-out below needs: the load is imbalanced by design (interior pixels
// run to maxIter, exterior ones exit after a few iterations), so a static
// contiguous split would measure the split rather than the backend.
func rowIterations(row, n, maxIter uint32, dx, dy float64) uint64 {
	ci := yMin + float64((float64(row)+0.5)*dy)
	var sum uint64

	for col := uint32(0); col < n; col++ {
		cr := xMin + float64((float64(col)+0.5)*dx)
		zr := 0.0
		zi := 0.0
		iter := uint32(0)

		for iter < maxIter {
			// Every float64() here is a rounding point, not a cast. See the top
			// of the file: without them the compiler fuses these into FMAs and
			// the checksum stops matching the rest of the table.
			zr2 := float64(zr * zr)
			zi2 := float64(zi * zi)
			if zr2+zi2 > 4.0 {
				break
			}
			zi = float64(2.0*zr*zi) + ci
			zr = zr2 - zi2 + cr
			iter++
		}
		sum += uint64(iter)
	}
	return sum
}

func parsePositive(text, name string) uint32 {
	value, err := strconv.ParseUint(text, 10, 32)
	if err != nil || value == 0 {
		fmt.Fprintf(os.Stderr, "%s must be a positive integer, got `%s`\n", name, text)
		os.Exit(2)
	}
	return uint32(value)
}

func main() {
	if len(os.Args) != 4 {
		fmt.Fprintf(os.Stderr, "usage: %s <n> <max_iter> <threads>\n", os.Args[0])
		os.Exit(2)
	}

	// Never compile-time constants: a backend would fold the whole computation
	// away and the benchmark would measure nothing, very quickly.
	n := parsePositive(os.Args[1], "n")
	maxIter := parsePositive(os.Args[2], "max_iter")
	threads := parsePositive(os.Args[3], "threads")

	// The kernel never asks the machine how many CPUs it has. Go would happily
	// answer -- and since 1.25 it answers from the cgroup quota, which is a
	// perfectly good answer to a question we refuse to ask: runtimes disagree
	// about that quota, and auto-detection would measure the disagreement. The
	// harness decides, argv carries the decision, and GOMAXPROCS is told.
	runtime.GOMAXPROCS(int(threads))

	dx := (xMax - xMin) / float64(n)
	dy := (yMax - yMin) / float64(n)

	var nextRow atomic.Uint32
	sums := make([]uint64, threads)
	var pool sync.WaitGroup

	// Goroutine creation is inside the timer on purpose: spawning the workers is
	// part of what a parallel runtime costs, and the point is to compare runtimes.
	// Go's are cheap, and that is a result rather than a reason to hide them.
	started := time.Now()

	for i := uint32(0); i < threads; i++ {
		pool.Add(1)
		go func(worker uint32) {
			defer pool.Done()
			var sum uint64
			for {
				// Add returns the value *after* the increment, where C's
				// fetch_add returns the value before it. Hence the -1: the
				// cursor's old value is this worker's row.
				row := nextRow.Add(1) - 1
				if row >= n {
					break
				}
				sum += rowIterations(row, n, maxIter, dx, dy)
			}
			sums[worker] = sum
		}(i)
	}
	pool.Wait()
	elapsed := time.Since(started)

	// Summing 64-bit integers is associative, so the reduction order cannot
	// perturb the checksum however the goroutines happened to finish.
	var checksum uint64
	for _, sum := range sums {
		checksum += sum
	}

	// Printing the checksum is what stops dead-code elimination from deleting the
	// loop above.
	fmt.Printf("%d %d\n", checksum, elapsed.Nanoseconds())
}
