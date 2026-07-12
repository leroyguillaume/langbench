// The table `report.md` prints, with the sort and the filters put back in the
// reader's hands.
//
// The row is named the way the report names it — three columns, `Language`,
// `Compiler`, `Interpreter` — and never by a slug. An absence is a published fact
// (`n/a` in the Interpreter column means this backend ships machine code and no
// runtime), so it is rendered rather than blanked: a blank cell reads as a
// rendering bug.
//
// Every column sorts on the *number*, never on the string it is spelled as:
// `"1000.0 ms"` sorts before `"2.0 ms"` alphabetically, and a table that does that
// is worse than no table. The formatting happens on the way out.

import type { Aggregate, FpMode, Summary } from "../analysis";
import {
  bytes,
  delta,
  dispersion,
  milliseconds,
  NOT_AVAILABLE,
  optional,
  ratio,
  seconds,
} from "../format";
import { ABSENT, anchorId, label, type Triple } from "../identity";
import { MODE_COLOR } from "../series";
import type { Filters } from "../url";

/** How suspect a dispersion has to be before the table says so out loud. */
const DISPERSION_CEILING = 2;

export type SortKey =
  | "language"
  | "compiler"
  | "interpreter"
  | "mode"
  | "runs"
  | "run"
  | "dispersion"
  | "compute"
  | "startup"
  | "cpu"
  | "build"
  | "build_dispersion"
  | "binary"
  | "text";

export interface Sort {
  key: SortKey;
  descending: boolean;
}

interface Column {
  key: SortKey;
  label: string;
  /** A column of the triple, not a measurement: it sorts as text and aligns left. */
  text?: boolean;
  /** The number this column sorts on. `null` — not measured — always sorts last. */
  value: (row: Aggregate) => number | string | null;
}

const min = (summary: Summary | null): number | null => summary?.min ?? null;

const COLUMNS: Column[] = [
  {
    key: "language",
    label: "Language",
    text: true,
    value: (row) => row.language,
  },
  {
    key: "compiler",
    label: "Compiler",
    text: true,
    value: (row) => row.compiler,
  },
  {
    key: "interpreter",
    label: "Interpreter",
    text: true,
    value: (row) => row.interpreter,
  },
  { key: "mode", label: "Mode", text: true, value: (row) => row.mode },
  { key: "runs", label: "Runs", value: (row) => row.run_wall?.n ?? null },
  { key: "run", label: "Run min", value: (row) => min(row.run_wall) },
  {
    key: "dispersion",
    label: "Dispersion",
    value: (row) => row.run_wall?.mad_pct ?? null,
  },
  {
    key: "compute",
    label: "Compute min",
    value: (row) => min(row.run_elapsed),
  },
  { key: "startup", label: "Startup", value: (row) => min(row.run_startup) },
  {
    key: "cpu",
    label: "CPU time",
    value: (row) => row.run_cpu_usec?.median ?? null,
  },
  { key: "build", label: "Build min", value: (row) => min(row.build_elapsed) },
  {
    key: "build_dispersion",
    label: "Build disp.",
    value: (row) => row.build_elapsed?.mad_pct ?? null,
  },
  { key: "binary", label: "Binary", value: (row) => row.binary_bytes },
  { key: "text", label: ".text", value: (row) => row.text_bytes },
];

/**
 * Does a row's field satisfy a filter? `null` — no filter — always yes.
 *
 * `ABSENT` is a filter like any other, and it selects the rows where the field is
 * genuinely missing: the ahead-of-time backends have no interpreter, and asking for
 * exactly those is a question, not a mistake.
 */
function matches(value: string | null, filter: string | null): boolean {
  if (filter === null) {
    return true;
  }
  return filter === ABSENT ? value === null : value === filter;
}

/**
 * What the filter bar leaves standing. Applied to the table, the charts and the
 * failures alike — a filter that only narrowed one of the three would be showing
 * three different campaigns on one page.
 *
 * The needle is matched against the triple and nothing else — a reader who types
 * `gcc` means the compiler, and matching them against an internal slug would hand
 * them rows for reasons they cannot see on screen.
 */
export function filterRows<T extends Triple & { mode: FpMode }>(rows: T[], filters: Filters): T[] {
  const needle = filters.search.trim().toLowerCase();
  return rows.filter((row) => {
    if (!filters.modes.includes(row.mode)) {
      return false;
    }
    if (
      !matches(row.language, filters.language) ||
      !matches(row.compiler, filters.compiler) ||
      !matches(row.interpreter, filters.interpreter)
    ) {
      return false;
    }
    if (needle === "") {
      return true;
    }
    return [row.language, row.compiler, row.interpreter]
      .filter((field): field is string => field !== null)
      .some((field) => field.toLowerCase().includes(needle));
  });
}

export function sortRows(rows: Aggregate[], sort: Sort): Aggregate[] {
  const column = COLUMNS.find((candidate) => candidate.key === sort.key);
  if (column === undefined) {
    return rows;
  }
  const direction = sort.descending ? -1 : 1;
  return [...rows].sort((left, right) => {
    const a = column.value(left);
    const b = column.value(right);
    // A row the campaign could not measure on this column has no rank on it — and
    // neither has a backend that has no compiler to sort by. It sorts last in
    // *both* directions rather than winning the ascending sort.
    if (a === null && b === null) {
      return 0;
    }
    if (a === null) {
      return 1;
    }
    if (b === null) {
      return -1;
    }
    if (typeof a === "string" || typeof b === "string") {
      return String(a).localeCompare(String(b)) * direction;
    }
    return (a - b) * direction;
  });
}

interface Props {
  rows: Aggregate[];
  sort: Sort;
  onSort: (key: SortKey) => void;
}

export function ResultsTable({ rows, sort, onSort }: Props) {
  // The ratio is against the fastest row *on screen*: the reader chose this set,
  // and a baseline drawn from rows they filtered out is a baseline they cannot see.
  const fastest = rows.reduce<number | null>((best, row) => {
    const value = min(row.run_wall);
    if (value === null) {
      return best;
    }
    return best === null || value < best ? value : best;
  }, null);

  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            {COLUMNS.map((column) => (
              <th
                key={column.key}
                className={column.text === true ? "text" : ""}
                onClick={() => onSort(column.key)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSort(column.key);
                  }
                }}
                aria-sort={
                  sort.key === column.key ? (sort.descending ? "descending" : "ascending") : "none"
                }
              >
                {column.label}{" "}
                <span className="sort-caret">
                  {sort.key === column.key ? (sort.descending ? "▼" : "▲") : ""}
                </span>
              </th>
            ))}
            {/* Not in `report.md`, and so not in the shared column reference: the
                ratio is against the fastest row *on screen*, and the report has no
                screen. Explained where it appears, above the table. */}
            <th className="text">Ratio</th>
            <th className="text">Δ strict</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const run = min(row.run_wall);
            const suspect =
              (row.run_wall?.n ?? 0) >= 3 && (row.run_wall?.mad_pct ?? 0) > DISPERSION_CEILING;
            return (
              <tr key={`${row.backend_id}-${row.mode}`}>
                {/* To the card that says what this thing *is*. A row of fifteen
                    numbers is not self-explanatory, and the implementation describes
                    itself — including the caveats about what its numbers do not say.
                    The report's Language cell links to the same place. */}
                <td className="text">
                  <a
                    className="row-link"
                    href={`#${anchorId(row)}`}
                    title={`what ${label(row)} is, and what its author wanted you to know`}
                  >
                    {row.language}
                  </a>
                </td>
                {/* `n/a` is the answer, not a missing one: a compiled backend has no
                    interpreter, and the manifest says so on purpose. */}
                <td className="text muted-cell">{optional(row.compiler)}</td>
                <td className="text muted-cell">{optional(row.interpreter)}</td>
                <td className="text">
                  <span className="mode-tag">
                    <span
                      className="mode-dot"
                      style={{ background: `var(${MODE_COLOR[row.mode]})` }}
                    />
                    {row.mode}
                  </span>
                </td>
                {/* How many measured samples went into this row. The warmup rounds
                    are in `samples.ndjson`, flagged, and out of these numbers. */}
                <td className="numeric">{row.run_wall?.n ?? NOT_AVAILABLE}</td>
                <td className="numeric">{milliseconds(run)}</td>
                <td className="numeric">
                  {/* Above ~2% the campaign cannot defend a percentage-level claim.
                      Flagged with a word and a background, never with colour alone. */}
                  <span
                    className={suspect ? "suspect" : ""}
                    title={
                      suspect
                        ? "above 2%: percentage-level claims are not defensible on this campaign"
                        : ""
                    }
                  >
                    {dispersion(row.run_wall)}
                  </span>
                </td>
                <td className="numeric">{milliseconds(min(row.run_elapsed))}</td>
                <td className="numeric">{milliseconds(min(row.run_startup))}</td>
                <td className="numeric">{seconds(row.run_cpu_usec?.median ?? null)}</td>
                <td className="numeric">{milliseconds(min(row.build_elapsed))}</td>
                <td className="numeric">{dispersion(row.build_elapsed)}</td>
                <td className="numeric">{bytes(row.binary_bytes)}</td>
                <td className="numeric">{bytes(row.text_bytes)}</td>
                <td className="numeric text">
                  {run !== null && fastest !== null ? ratio(run, fastest) : "n/a"}
                </td>
                <td className="numeric text">{delta(row.checksum_delta)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {rows.length === 0 && <p className="bar-empty">No implementation matches these filters.</p>}
    </div>
  );
}
