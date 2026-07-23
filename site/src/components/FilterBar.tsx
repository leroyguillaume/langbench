// The filters: which rows of *this* campaign to look at.
//
// Three of them are the three columns of the table — language, compiler, interpreter —
// because that is what an implementation *is*, and narrowing by "every backend whose
// name contains gcc" is narrowing by a string somebody typed. A reader who picks
// `python` and then opens the compiler list sees `cython` and nothing else: the
// options are the ones this campaign actually measured, never a fixed list that
// promises rows the file does not have.
//
// **The campaign is not among them.** Which workload, on which machine, is the page
// you are on — the sidebar navigates between campaigns, and a filter narrows one. A
// `<select>` that swapped the architecture would put two experiments behind one control
// and invite the reader to read across them.

import type { Aggregate, Mode } from "../analysis";
import { NOT_AVAILABLE } from "../format";
import { ABSENT } from "../identity";
import { MODES } from "../series";
import type { Filters } from "../url";

interface Props {
  filters: Filters;
  onFilters: (filters: Filters) => void;
  /** Every row of the campaign on screen, before any filter: the options come from these. */
  rows: Aggregate[];
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

export function FilterBar({ filters, onFilters, rows }: Props) {
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

  // The modes this campaign actually measured, in the canonical order — never the whole
  // enum. A campaign of JIT-only backends carries no `baseline` row, and a checkbox that
  // offers one is offering an empty table: the same rule the compiler and interpreter
  // lists already obey, and the reason they are read off the rows. The *order* and the
  // colour still come from `MODES`, because a series that reorders itself repaints the
  // chart.
  const measured = MODES.filter((mode) => rows.some((row) => row.mode === mode));

  const toggleMode = (mode: Mode) => {
    const modes = filters.modes.includes(mode)
      ? filters.modes.filter((candidate) => candidate !== mode)
      : MODES.filter((candidate) => filters.modes.includes(candidate) || candidate === mode);
    // Never leave the reader with an empty chart and no way back — and "empty" is judged
    // against the modes this campaign has rows in, not against the enum: keeping a mode
    // nothing was measured in would clear the table while every checkbox on screen stayed
    // unticked, which reads as a broken page.
    const survives = modes.some((candidate) => measured.includes(candidate));
    onFilters({ ...filters, modes: survives ? modes : MODES });
  };

  const dirty =
    filters.language !== null ||
    filters.compiler !== null ||
    filters.interpreter !== null ||
    filters.search !== "" ||
    filters.modes.length !== MODES.length;

  return (
    <div className="filters">
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
        <span>ISA target</span>
        <div className="chart-bars">
          {measured.map((mode) => (
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
