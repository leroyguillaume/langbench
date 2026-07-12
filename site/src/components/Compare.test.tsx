// What the panel is allowed to decide: which two rows, and how to spell them.
//
// Every verdict below arrives already made, out of `src/compare.rs`. The tests
// that check *whether a gap is a difference* are in Rust, where that decision
// lives; these check that a decision the harness made survives being rendered —
// and that a tie is never quietly drawn as a win.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Aggregate, Comparison, Metric, Summary } from "../analysis";
import { Compare, resolve, rowKey } from "./Compare";

const summary = (min: number, overrides: Partial<Summary> = {}): Summary => ({
  n: 10,
  min,
  median: min,
  mad: 0,
  mad_pct: 0,
  ...overrides,
});

function aggregate(backend: string, runMin: number | null): Aggregate {
  return {
    algo: "mandelbrot",
    backend,
    backend_id: `mandelbrot-${backend}`,
    language: backend.split("-")[0] ?? backend,
    compiler: null,
    interpreter: null,
    mode: "strict",
    run_wall: runMin === null ? null : summary(runMin),
    run_elapsed: null,
    run_startup: null,
    run_cpu_usec: null,
    build_elapsed: null,
    binary_bytes: null,
    binary_stripped_bytes: null,
    text_bytes: null,
    checksum: "42",
    checksum_delta: "0",
  };
}

function metric(overrides: Partial<Metric> = {}): Metric {
  return {
    key: "run",
    label: "Run (external wall-clock)",
    unit: "nanoseconds",
    left: 2_000_000_000,
    right: 3_000_000_000,
    ratio: 1.5,
    gap_pct: 50,
    noise_pct: 0.4,
    verdict: "left",
    ...overrides,
  };
}

function comparison(overrides: Partial<Comparison> = {}): Comparison {
  return {
    algo: "mandelbrot",
    left: {
      backend: "c-gcc",
      backend_id: "mandelbrot-c-gcc",
      language: "c",
      compiler: "gcc",
      interpreter: null,
      mode: "strict",
    },
    right: {
      backend: "c-clang",
      backend_id: "mandelbrot-c-clang",
      language: "c",
      compiler: "clang",
      interpreter: null,
      mode: "strict",
    },
    metrics: [metric()],
    checksums: {
      left: "42",
      right: "42",
      same: true,
      violates_strict_invariant: false,
    },
    ...overrides,
  };
}

describe("picking the pair", () => {
  const rows = [aggregate("c-gcc", 1_000_000_000), aggregate("c-clang", 2_000_000_000)];

  it("pairs the winner with its runner-up when the reader has picked nothing", () => {
    expect(resolve(rows, "mandelbrot", null, null)).toStrictEqual({
      algo: "mandelbrot",
      left: { backend: "c-gcc", mode: "strict" },
      right: { backend: "c-clang", mode: "strict" },
    });
  });

  /// A stale bookmark names a backend this campaign never ran. Dropping the row is
  /// the whole page's alternative to refusing to render.
  it("drops a row this campaign never measured rather than passing it to the harness", () => {
    const selection = resolve(
      rows,
      "mandelbrot",
      { backend: "fortran-gfortran", mode: "strict" },
      { backend: "c-clang", mode: "strict" },
    );
    expect(selection?.left).toStrictEqual({ backend: "c-gcc", mode: "strict" });
    expect(selection?.right).toStrictEqual({ backend: "c-clang", mode: "strict" });
  });

  it("has no pair to offer below two measured rows", () => {
    expect(resolve([aggregate("c-gcc", 1_000_000_000)], "mandelbrot", null, null)).toBeNull();
    expect(resolve([], "mandelbrot", null, null)).toBeNull();
  });

  it("names a row the way the URL and the samples do", () => {
    expect(rowKey({ backend: "python-cython-cpython", mode: "fast" })).toBe(
      "python-cython-cpython:fast",
    );
  });
});

describe("the panel", () => {
  const rows = [aggregate("c-gcc", 1_000_000_000), aggregate("c-clang", 2_000_000_000)];
  const selection = resolve(rows, "mandelbrot", null, null);

  const panel = (value: Comparison) =>
    render(
      <Compare
        aggregates={rows}
        selection={selection}
        comparison={value}
        error={null}
        onSelect={() => {}}
      />,
    );

  it("names the winner and the gap, on the metric the report headlines", () => {
    panel(comparison());
    expect(screen.getByText(/is faster by/)).toHaveTextContent("c-gcc");
    expect(screen.getByText("50.0%")).toBeInTheDocument();
    expect(screen.getByText("1.50×")).toBeInTheDocument();
  });

  /// The reason the whole panel exists. Two minima that differ, on a campaign in
  /// no position to say which is smaller: the answer is "we cannot tell", and it
  /// is said out loud rather than left to be inferred from a 1.03× in a column.
  it("calls a gap smaller than the noise indistinguishable, never a win", () => {
    panel(
      comparison({
        metrics: [
          metric({
            left: 1_000_000_000,
            right: 1_030_000_000,
            ratio: 1.03,
            gap_pct: 3,
            noise_pct: 9,
            verdict: "tie",
          }),
        ],
      }),
    );
    // Twice, and deliberately: the headline states the verdict, the row carries it.
    expect(screen.getAllByText(/indistinguishable/)).toHaveLength(2);
    expect(screen.queryByText(/is faster by/)).not.toBeInTheDocument();
  });

  it("says an absent number is absent, and ranks nothing on it", () => {
    panel(
      comparison({
        metrics: [
          metric({
            key: "binary",
            label: "Binary size",
            unit: "bytes",
            left: 20_480,
            right: null,
            ratio: null,
            gap_pct: null,
            noise_pct: null,
            verdict: "unmeasured",
          }),
        ],
      }),
    );
    expect(screen.getByText("20.0 KiB")).toBeInTheDocument();
    expect(screen.getByText("one side has no such number")).toBeInTheDocument();
  });

  /// Two strict rows that disagree are a bug, never a rounding excuse -- and a
  /// timing beside a wrong answer is not a result.
  it("says so, loudly, when two strict rows disagree on the checksum", () => {
    panel(
      comparison({
        checksums: {
          left: "9007199254740993",
          right: "9007199254740994",
          same: false,
          violates_strict_invariant: true,
        },
      }),
    );
    expect(screen.getByText(/not comparable/)).toBeInTheDocument();
    // The full 64 bits, to the last digit: this is exactly what `JSON.parse` would
    // have rounded away.
    expect(screen.getByText("9007199254740993")).toBeInTheDocument();
  });

  it("explains a divergence a relaxed mode was always going to produce", () => {
    panel(
      comparison({
        right: {
          backend: "c-gcc",
          backend_id: "mandelbrot-c-gcc",
          language: "c",
          compiler: "gcc",
          interpreter: null,
          mode: "fast",
        },
        checksums: { left: "1000", right: "994", same: false, violates_strict_invariant: false },
      }),
    );
    expect(screen.getByText(/Expected/)).toBeInTheDocument();
  });
});
