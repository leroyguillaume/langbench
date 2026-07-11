// The table `report.md` prints, with the sort put back in the reader's hands.
//
// Every column sorts on the *number*, never on the string it is spelled as:
// `"1000.0 ms"` sorts before `"2.0 ms"` alphabetically, and a table that does
// that is worse than no table. The formatting happens on the way out.

import type { Aggregate, Summary } from "../analysis";
import { bytes, delta, dispersion, milliseconds, optional, ratio, seconds } from "../format";
import { MODE_COLOR } from "../series";

/** How suspect a dispersion has to be before the table says so out loud. */
const DISPERSION_CEILING = 2;

export type SortKey =
  | "backend"
  | "mode"
  | "run"
  | "dispersion"
  | "compute"
  | "startup"
  | "cpu"
  | "build"
  | "binary"
  | "text";

export interface Sort {
  key: SortKey;
  descending: boolean;
}

interface Column {
  key: SortKey;
  label: string;
  /** The number this column sorts on. `null` — not measured — always sorts last. */
  value: (row: Aggregate) => number | string | null;
}

const min = (summary: Summary | null): number | null => summary?.min ?? null;

const COLUMNS: Column[] = [
  { key: "backend", label: "Backend", value: (row) => row.backend },
  { key: "mode", label: "Mode", value: (row) => row.mode },
  { key: "run", label: "Run (min)", value: (row) => min(row.run_wall) },
  { key: "dispersion", label: "±", value: (row) => row.run_wall?.mad_pct ?? null },
  { key: "compute", label: "Compute", value: (row) => min(row.run_elapsed) },
  { key: "startup", label: "Startup", value: (row) => min(row.run_startup) },
  { key: "cpu", label: "CPU time", value: (row) => row.run_cpu_usec?.median ?? null },
  { key: "build", label: "Build", value: (row) => min(row.build_elapsed) },
  { key: "binary", label: "Binary", value: (row) => row.binary_bytes },
  { key: "text", label: ".text", value: (row) => row.text_bytes },
];

export function sortRows(rows: Aggregate[], sort: Sort): Aggregate[] {
  const column = COLUMNS.find((candidate) => candidate.key === sort.key);
  if (column === undefined) {
    return rows;
  }
  const direction = sort.descending ? -1 : 1;
  return [...rows].sort((left, right) => {
    const a = column.value(left);
    const b = column.value(right);
    // A backend the campaign could not measure on this column has no rank on it.
    // It sorts last in *both* directions rather than winning the ascending sort.
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
                className={column.key === "backend" || column.key === "mode" ? "text" : ""}
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
            <th className="text">Ratio</th>
            <th className="text">Δ checksum</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const run = min(row.run_wall);
            const suspect =
              (row.run_wall?.n ?? 0) >= 3 && (row.run_wall?.mad_pct ?? 0) > DISPERSION_CEILING;
            return (
              <tr key={`${row.backend_id}-${row.mode}`}>
                <td
                  className="text"
                  title={`${row.language} · ${optional(row.compiler)} · ${optional(row.interpreter)}`}
                >
                  {row.backend}
                </td>
                <td className="text">
                  <span className="mode-tag">
                    <span
                      className="mode-dot"
                      style={{ background: `var(${MODE_COLOR[row.mode]})` }}
                    />
                    {row.mode}
                  </span>
                </td>
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
      {rows.length === 0 && <p className="bar-empty">No backend matches these filters.</p>}
    </div>
  );
}
