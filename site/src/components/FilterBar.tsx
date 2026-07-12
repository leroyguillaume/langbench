// The filters, and what they scope.
//
// Three of them are the three columns of the report — language, compiler,
// interpreter — because that is what an implementation *is*, and narrowing by
// "every backend whose name contains gcc" is narrowing by a string somebody typed.
// A reader who picks `python` and then opens the compiler list sees `cython` and
// nothing else: the options are the ones this campaign actually measured, never a
// fixed list that promises rows the file does not have.
//
// The ISA comes first, and it is not a filter: it is the experiment the rest of the
// page is about. An absolute timing does not cross an ISA.

import type { Aggregate, FpMode } from "../analysis";
import { NOT_AVAILABLE } from "../format";
import { ABSENT } from "../identity";
import { MODES } from "../series";
import type { Filters, Scope } from "../url";

interface Props {
  scope: Scope;
  onScope: (scope: Scope) => void;
  filters: Filters;
  onFilters: (filters: Filters) => void;
  /** Every row of the algorithm on screen, before any filter: the options come from these. */
  rows: Aggregate[];
  algos: string[];
  arches: string[];
  arch: string;
}

/**
 * The values a field takes in this campaign, in order, with `null` spelled out.
 *
 * `null` is offered as a choice rather than dropped, because "the implementations
 * with no interpreter" is a question with an answer — it is every ahead-of-time
 * backend in the table.
 */
function options(rows: Aggregate[], field: "compiler" | "interpreter"): (string | null)[] {
  const values = new Set(rows.map((row) => row[field]));
  const named = [...values].filter((value): value is string => value !== null).sort();
  return values.has(null) ? [...named, null] : named;
}

/**
 * What a `<select>` says, as a filter.
 *
 * `""` is the empty option — no filter at all. Anything else is a filter, and that
 * includes `ABSENT`: "the ones with no compiler" narrows the table exactly as
 * "the ones compiled by gcc" does.
 */
function decode(raw: string): string | null {
  return raw === "" ? null : raw;
}

/** The value a chosen field puts in the `<select>` and in the URL. */
function encode(value: string | null): string {
  return value ?? "";
}

export function FilterBar({
  scope,
  onScope,
  filters,
  onFilters,
  rows,
  algos,
  arches,
  arch,
}: Props) {
  const languages = [...new Set(rows.map((row) => row.language))].sort();

  // The compiler and interpreter lists are scoped by the language already chosen:
  // offering `gcc` to somebody who selected `python` is offering an empty table.
  const scoped =
    filters.language === null ? rows : rows.filter((row) => row.language === filters.language);
  const compilers = options(scoped, "compiler");
  const interpreters = options(scoped, "interpreter");

  // Changing the language drops a compiler that language never had, rather than
  // leaving the reader on an empty table with no visible reason why. `python` and
  // `gcc` is not a narrower question than `python` — it is an empty one.
  const setLanguage = (language: string | null) => {
    const next = language === null ? rows : rows.filter((row) => row.language === language);
    const keep = (filter: string | null, field: "compiler" | "interpreter"): string | null => {
      if (filter === null) {
        return null;
      }
      const survives = next.some((row) =>
        filter === ABSENT ? row[field] === null : row[field] === filter,
      );
      return survives ? filter : null;
    };
    onFilters({
      ...filters,
      language,
      compiler: keep(filters.compiler, "compiler"),
      interpreter: keep(filters.interpreter, "interpreter"),
    });
  };

  const toggleMode = (mode: FpMode) => {
    const modes = filters.modes.includes(mode)
      ? filters.modes.filter((candidate) => candidate !== mode)
      : MODES.filter((candidate) => filters.modes.includes(candidate) || candidate === mode);
    // Never leave the reader with an empty chart and no way back.
    onFilters({ ...filters, modes: modes.length === 0 ? MODES : modes });
  };

  const dirty =
    filters.language !== null ||
    filters.compiler !== null ||
    filters.interpreter !== null ||
    filters.search !== "" ||
    filters.modes.length !== MODES.length;

  return (
    <div className="filters">
      {/* First, because it scopes everything below it. A campaign is not a filter
          over the others: it is the experiment the rest of the page is about. */}
      {arches.length > 1 && (
        <label className="filter">
          <span>ISA</span>
          <select
            value={arch}
            onChange={(event) => onScope({ ...scope, arch: event.target.value })}
          >
            {arches.map((candidate) => (
              <option key={candidate} value={candidate}>
                {candidate}
              </option>
            ))}
          </select>
        </label>
      )}

      <label className="filter">
        <span>Algorithm</span>
        <select
          value={scope.algo ?? ""}
          onChange={(event) => onScope({ ...scope, algo: event.target.value })}
        >
          {algos.map((algo) => (
            <option key={algo} value={algo}>
              {algo}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Language</span>
        <select
          value={encode(filters.language)}
          onChange={(event) => setLanguage(decode(event.target.value))}
        >
          <option value="">every language</option>
          {languages.map((language) => (
            <option key={language} value={language}>
              {language}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Compiler</span>
        <select
          value={encode(filters.compiler)}
          onChange={(event) => onFilters({ ...filters, compiler: decode(event.target.value) })}
        >
          <option value="">every compiler</option>
          {compilers.map((compiler) => (
            <option key={compiler ?? ABSENT} value={compiler ?? ABSENT}>
              {compiler ?? `${NOT_AVAILABLE} — compiles nothing ahead of the run`}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Interpreter</span>
        <select
          value={encode(filters.interpreter)}
          onChange={(event) => onFilters({ ...filters, interpreter: decode(event.target.value) })}
        >
          <option value="">every interpreter</option>
          {interpreters.map((interpreter) => (
            <option key={interpreter ?? ABSENT} value={interpreter ?? ABSENT}>
              {interpreter ?? `${NOT_AVAILABLE} — ships machine code, no runtime`}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Search</span>
        <input
          type="search"
          placeholder="gcc, cpython, kotlin…"
          value={filters.search}
          onChange={(event) => onFilters({ ...filters, search: event.target.value })}
        />
      </label>

      <div className="filter">
        <span>FP mode</span>
        <div className="chart-bars">
          {MODES.map((mode) => (
            <label className="toggle" key={mode}>
              <input
                type="checkbox"
                checked={filters.modes.includes(mode)}
                onChange={() => toggleMode(mode)}
              />
              {mode}
            </label>
          ))}
        </div>
      </div>

      {dirty && (
        <button
          type="button"
          className="filter-clear"
          onClick={() =>
            onFilters({
              language: null,
              compiler: null,
              interpreter: null,
              search: "",
              modes: MODES,
            })
          }
        >
          clear filters
        </button>
      )}
    </div>
  );
}
