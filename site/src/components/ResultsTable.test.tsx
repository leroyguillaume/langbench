import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Aggregate, Summary } from "../analysis";
import { ResultsTable, sortRows } from "./ResultsTable";

const summary = (min: number, overrides: Partial<Summary> = {}): Summary => ({
  n: 10,
  min,
  median: min,
  mad: 0,
  mad_pct: 0,
  ...overrides,
});

function row(
  backend: string,
  runMin: number | null,
  overrides: Partial<Aggregate> = {},
): Aggregate {
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
    ...overrides,
  };
}

describe("sorting", () => {
  it("sorts on the number, not on the string it is spelled as", () => {
    // "1000.0 ms" sorts before "2.0 ms" alphabetically. A table that does that is
    // worse than no table.
    const rows = [row("slow", 1_000_000_000), row("quick", 2_000_000)];
    const sorted = sortRows(rows, { key: "run", descending: false });
    expect(sorted.map((entry) => entry.backend)).toStrictEqual(["quick", "slow"]);
  });

  it("sends a row the campaign could not measure last, in both directions", () => {
    const rows = [row("unmeasured", null), row("measured", 5_000_000)];

    expect(
      sortRows(rows, { key: "run", descending: false }).map((entry) => entry.backend),
    ).toStrictEqual(["measured", "unmeasured"]);
    // Descending too: an absent number is not an infinitely large one.
    expect(
      sortRows(rows, { key: "run", descending: true }).map((entry) => entry.backend),
    ).toStrictEqual(["measured", "unmeasured"]);
  });
});

describe("the table", () => {
  it("ratios every row against the fastest one on screen", () => {
    render(
      <ResultsTable
        rows={[row("c-gcc", 1_000_000_000), row("python-cpython", 13_000_000_000)]}
        sort={{ key: "run", descending: false }}
        onSort={() => {}}
      />,
    );
    expect(screen.getByText("1.00×")).toBeInTheDocument();
    expect(screen.getByText("13.0×")).toBeInTheDocument();
  });

  /// A dispersion the campaign cannot defend is said out loud, not left for the
  /// reader to notice — and never flagged by colour alone.
  it("calls out a dispersion above 2%", () => {
    render(
      <ResultsTable
        rows={[
          row("noisy", 1_000_000_000, {
            run_wall: summary(1_000_000_000, { mad_pct: 7.4, n: 10 }),
          }),
        ]}
        sort={{ key: "run", descending: false }}
        onSort={() => {}}
      />,
    );
    const cell = screen.getByText("7.40%");
    expect(cell).toHaveClass("suspect");
    expect(cell).toHaveAttribute("title", expect.stringContaining("not defensible"));
  });

  it("reports a two-sample dispersion as unavailable rather than as zero", () => {
    // The lower median of `[0, d]` is `0`: a two-sample MAD is structurally zero
    // and would claim a precision the campaign never had.
    render(
      <ResultsTable
        rows={[row("thin", 1_000_000_000, { run_wall: summary(1_000_000_000, { n: 2 }) })]}
        sort={{ key: "run", descending: false }}
        onSort={() => {}}
      />,
    );
    expect(screen.getByText("n/a (n=2)")).toBeInTheDocument();
  });
});
