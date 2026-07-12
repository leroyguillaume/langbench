import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Aggregate, Summary } from "../analysis";
import { NO_FILTERS } from "../url";
import { filterRows, ResultsTable, sortRows } from "./ResultsTable";

const summary = (min: number, overrides: Partial<Summary> = {}): Summary => ({
  n: 10,
  min,
  median: min,
  mad: 0,
  mad_pct: 0,
  ...overrides,
});

/**
 * A row, named by its triple — the way a manifest declares it and the way the
 * report prints it. The `backend` slug is still on the wire, because the WASM picks
 * rows by it; nothing below reads it.
 */
function row(
  triple: {
    language: string;
    compiler?: string | null;
    interpreter?: string | null;
  },
  runMin: number | null,
  overrides: Partial<Aggregate> = {},
): Aggregate {
  const compiler = triple.compiler ?? null;
  const interpreter = triple.interpreter ?? null;
  const slug = [triple.language, compiler, interpreter].filter(Boolean).join("-");
  return {
    algo: "mandelbrot",
    backend: slug,
    backend_id: `mandelbrot-${slug}`,
    language: triple.language,
    compiler,
    interpreter,
    mode: "strict",
    run_wall: runMin === null ? null : summary(runMin),
    run_elapsed: null,
    run_startup: null,
    run_cpu_usec: null,
    run_cores: null,
    run_peak_bytes: null,
    run_energy_uj: null,
    build_cores: null,
    build_peak_bytes: null,
    build_energy_uj: null,
    cpu: 8,
    source_bytes: null,
    build_elapsed: null,
    binary_bytes: null,
    binary_stripped_bytes: null,
    text_bytes: null,
    checksum: "42",
    checksum_delta: "0",
    ...overrides,
  };
}

const C_GCC = row({ language: "c", compiler: "gcc" }, 1_000_000_000);
const PYTHON = row({ language: "python", interpreter: "cpython" }, 13_000_000_000);
const CYTHON = row(
  { language: "python", compiler: "cython", interpreter: "cpython" },
  3_000_000_000,
);

describe("sorting", () => {
  it("sorts on the number, not on the string it is spelled as", () => {
    // "1000.0 ms" sorts before "2.0 ms" alphabetically. A table that does that is
    // worse than no table.
    const rows = [row({ language: "slow" }, 1_000_000_000), row({ language: "quick" }, 2_000_000)];
    const sorted = sortRows(rows, { key: "run", descending: false });
    expect(sorted.map((entry) => entry.language)).toStrictEqual(["quick", "slow"]);
  });

  it("sends a row the campaign could not measure last, in both directions", () => {
    const rows = [row({ language: "unmeasured" }, null), row({ language: "measured" }, 5_000_000)];

    expect(
      sortRows(rows, { key: "run", descending: false }).map((entry) => entry.language),
    ).toStrictEqual(["measured", "unmeasured"]);
    // Descending too: an absent number is not an infinitely large one.
    expect(
      sortRows(rows, { key: "run", descending: true }).map((entry) => entry.language),
    ).toStrictEqual(["measured", "unmeasured"]);
  });

  // A backend with no compiler has no rank on the compiler column — it is not the
  // alphabetically-first compiler.
  it("sends a row with no compiler last when sorting by compiler", () => {
    const sorted = sortRows([PYTHON, C_GCC], {
      key: "compiler",
      descending: false,
    });
    expect(sorted.map((entry) => entry.compiler)).toStrictEqual(["gcc", null]);
  });
});

describe("filtering", () => {
  it("narrows on a field of the triple, not on a name somebody typed", () => {
    const rows = [C_GCC, PYTHON, CYTHON];
    expect(filterRows(rows, { ...NO_FILTERS, language: "python" })).toStrictEqual([PYTHON, CYTHON]);
    expect(filterRows(rows, { ...NO_FILTERS, compiler: "cython" })).toStrictEqual([CYTHON]);
  });

  // "Every compiler" and "the ones with no compiler" are different questions, and
  // the second one has an answer: every interpreted backend in the table.
  it("can ask for an absence, because an absence is a published fact", () => {
    expect(filterRows([C_GCC, PYTHON, CYTHON], { ...NO_FILTERS, compiler: "-" })).toStrictEqual([
      PYTHON,
    ]);
    expect(filterRows([C_GCC, PYTHON, CYTHON], { ...NO_FILTERS, interpreter: "-" })).toStrictEqual([
      C_GCC,
    ]);
  });

  it("searches the triple, and nothing but the triple", () => {
    expect(filterRows([C_GCC, PYTHON, CYTHON], { ...NO_FILTERS, search: "GCC" })).toStrictEqual([
      C_GCC,
    ]);
    expect(filterRows([C_GCC, PYTHON, CYTHON], { ...NO_FILTERS, search: "cpython" })).toStrictEqual(
      [PYTHON, CYTHON],
    );
  });

  it("drops a mode the reader turned off", () => {
    expect(filterRows([C_GCC], { ...NO_FILTERS, modes: ["fast"] })).toStrictEqual([]);
  });
});

describe("the table", () => {
  it("names a row by its triple, and spells the absent half rather than blanking it", () => {
    render(
      <ResultsTable
        rows={[C_GCC, PYTHON]}
        sort={{ key: "run", descending: false }}
        onSort={() => {}}
      />,
    );
    expect(screen.getByText("gcc")).toBeInTheDocument();
    expect(screen.getByText("cpython")).toBeInTheDocument();
    // c has no interpreter, python has no compiler: two facts, two cells of the
    // triple — and `n/a` in an unmeasured *metric* cell is a different statement,
    // so the count is taken on the triple's cells alone.
    const absent = screen
      .getAllByText("n/a")
      .filter((cell) => cell.classList.contains("muted-cell"));
    expect(absent).toHaveLength(2);
    // And never the slug.
    expect(screen.queryByText("c-gcc")).not.toBeInTheDocument();
  });

  it("ratios every row against the fastest one on screen", () => {
    render(
      <ResultsTable
        rows={[C_GCC, PYTHON]}
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
          row({ language: "noisy" }, 1_000_000_000, {
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
        rows={[
          row({ language: "thin" }, 1_000_000_000, {
            run_wall: summary(1_000_000_000, { n: 2 }),
          }),
        ]}
        sort={{ key: "run", descending: false }}
        onSort={() => {}}
      />,
    );
    expect(screen.getByText("n/a (n=2)")).toBeInTheDocument();
  });
});
