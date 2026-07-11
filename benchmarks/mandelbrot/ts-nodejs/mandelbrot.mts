// Mandelbrot: each pixel of an N x N grid maps onto a fixed viewport of the
// complex plane and iterates z <- z^2 + c until |z| > 2 or max_iter is reached.
//
// The program prints two integers on stdout: the checksum (the sum of every
// pixel's iteration count) and the wall-clock nanoseconds spent computing it.
//
// THE CHECKSUM IS THE HARNESS'S CORRECTNESS GATE. In strict floating-point mode
// it must be bit-identical to every other implementation's, C included. That
// holds because a TypeScript `number` is an IEEE 754 double, because multiply,
// add, subtract and compare are correctly rounded, and because no JavaScript
// engine may contract a multiply-add into an FMA or reassociate float arithmetic.
// It also requires the arithmetic below to be evaluated in exactly the order the
// C kernel uses: do not "simplify" it.
// See METHODOLOGY.md#the-strict-mode-invariant.
//
// WHAT THIS ROW IS FOR. TypeScript's types are erased, not compiled: Node, Deno
// and Bun each strip them and run the JavaScript underneath. So the honest
// expectation is that this row's compute time equals the `js` row's exactly, and
// the experiment is whether anything else moves -- startup, and the ahead-of-run
// check. A row that merely confirms an expectation is still a row worth having;
// "TypeScript is slower than JavaScript" is a claim people make.
//
// The types below are annotations only. No enum, no namespace, no parameter
// property: Node strips types syntactically and refuses anything it would have to
// *generate* code for, and the kernel stays inside that intersection so that one
// file serves all three runtimes, byte for byte.

import {
  Worker,
  isMainThread,
  parentPort,
  workerData,
} from "node:worker_threads";

// The viewport. Part of the cross-implementation contract: changing any of these
// constants changes the reference checksum.
const X_MIN = -2.0;
const X_MAX = 0.5;
const Y_MIN = -1.25;
const Y_MAX = 1.25;

// What every worker needs to know. `cursor` is the shared row queue; everything
// else is copied into each worker by the structured clone.
interface Grid {
  n: number;
  maxIter: number;
  dx: number;
  dy: number;
  cursor: SharedArrayBuffer;
}

// Sum the iteration counts of one row. The unit of work.
//
// With any realistic n there are far more rows than threads, which is what the
// dynamic hand-out below needs: the load is imbalanced by design (interior pixels
// run to max_iter, exterior ones exit after a few iterations), so a static
// contiguous split would measure the split rather than the backend.
function rowIterations(
  row: number,
  n: number,
  maxIter: number,
  dx: number,
  dy: number,
): number {
  const ci = Y_MIN + (row + 0.5) * dy;
  let sum = 0;

  for (let col = 0; col < n; ++col) {
    const cr = X_MIN + (col + 0.5) * dx;
    let zr = 0.0;
    let zi = 0.0;
    let iter = 0;

    while (iter < maxIter) {
      const zr2 = zr * zr;
      const zi2 = zi * zi;
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

// One worker's whole life: take the next row until there are none left.
//
// `Atomics.add` on a `SharedArrayBuffer` is this language's `fetch_add`, and the
// buffer is the only memory the workers share.
function work({ n, maxIter, dx, dy, cursor }: Grid): number {
  const nextRow = new Int32Array(cursor);
  let sum = 0;

  for (;;) {
    const row = Atomics.add(nextRow, 0, 1);
    if (row >= n) {
      break;
    }
    sum += rowIterations(row, n, maxIter, dx, dy);
  }
  return sum;
}

function parsePositive(text: string, name: string): number {
  const value = Number(text);
  if (!Number.isSafeInteger(value) || value <= 0) {
    console.error(`${name} must be a positive integer, got \`${text}\``);
    process.exit(2);
  }
  return value;
}

async function main(): Promise<void> {
  // `process.argv` and not `Deno.args` or `Bun.argv`: the three runtimes agree on
  // the Node spelling, and disagreeing here would cost us the shared kernel.
  const args = process.argv.slice(2);
  if (args.length !== 3) {
    console.error("usage: mandelbrot.mts <n> <max_iter> <threads>");
    process.exit(2);
  }

  // Never module-level constants: a backend could fold the computation away.
  const n = parsePositive(args[0], "n");
  const maxIter = parsePositive(args[1], "max_iter");
  const threads = parsePositive(args[2], "threads");

  const grid: Grid = {
    n,
    maxIter,
    dx: (X_MAX - X_MIN) / n,
    dy: (Y_MAX - Y_MIN) / n,
    cursor: new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT),
  };

  // Spawning the workers is inside the timer on purpose: what a parallel runtime
  // costs to start is part of what that runtime costs, and a `worker_threads`
  // worker boots a whole new isolate -- which, here, must strip the types again.
  const started = process.hrtime.bigint();
  const sums = await Promise.all(
    Array.from(
      { length: threads },
      () =>
        new Promise<number>((resolve, reject) => {
          const worker = new Worker(new URL(import.meta.url), {
            workerData: grid,
          });
          worker.on("message", resolve);
          worker.on("error", reject);
        }),
    ),
  );
  const elapsedNs = process.hrtime.bigint() - started;

  // Summing integers is associative, so the order in which the workers finish
  // cannot perturb the checksum.
  const checksum = sums.reduce((total, sum) => total + sum, 0);

  // The contract says the checksum is a 64-bit integer, and this is the one
  // language in the table that cannot store one: a number is a double, exact only
  // up to 2^53. It has room to spare -- n^2 * max_iter is ~1.6e10 at the campaign
  // size, five orders of magnitude below the limit -- but "has room to spare" is
  // not an invariant, so say so out loud rather than publish a rounded checksum.
  if (!Number.isSafeInteger(checksum)) {
    console.error(`checksum ${checksum} exceeds 2^53 and is no longer exact`);
    process.exit(1);
  }

  // Printing the checksum is what stops the JIT from eliding the loop above.
  console.log(`${checksum} ${elapsedNs}`);
}

if (isMainThread) {
  await main();
} else {
  parentPort!.postMessage(work(workerData as Grid));
}
