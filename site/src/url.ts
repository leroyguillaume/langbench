// The view lives in the URL.
//
// A filtered, sorted table is a claim about the data, and a claim you cannot link
// to is a claim nobody can check. So every knob round-trips through the query
// string: the address bar always describes what is on screen.
//
// The query string is an I/O boundary like any other — it is validated, never
// trusted. `?mode=strict,rubbish` narrows to `strict`; `?sort=drop_table` falls
// back to the default rather than reaching the sort with a key it has no column for.
//
// Three pages, one vocabulary: `arch`, `algo` and `warmup` mean the same thing on
// each of them, and `compareHref` carries them across so that clicking "Compare"
// from an aarch64 campaign does not silently land on the x86-64 one.

import { z } from "zod";
import { type FpMode, fpModeSchema } from "./analysis";
import type { Sort, SortKey } from "./components/ResultsTable";
import { MODES } from "./series";

const sortKeySchema = z.enum([
  "language",
  "compiler",
  "interpreter",
  "mode",
  "runs",
  "run",
  "dispersion",
  "compute",
  "startup",
  "cpu",
  "cores",
  "memory",
  "energy",
  "build",
  "build_dispersion",
  "source",
  "binary",
  "text",
]);

/** What both pages agree about: which campaign, which algorithm, and how it was aggregated. */
export interface Scope {
  /** The ISA whose campaign is on screen. `null` — whichever one sorts first. */
  arch: string | null;
  algo: string | null;
  /** Warmup rounds are always recorded. This decides whether they are aggregated. */
  includeWarmup: boolean;
}

/** Everything the results table narrows by. Each is a column of the report. */
export interface Filters {
  language: string | null;
  compiler: string | null;
  interpreter: string | null;
  /** A free-text needle, matched against the triple — never against a slug. */
  search: string;
  modes: FpMode[];
}

export interface ResultsState extends Scope {
  filters: Filters;
  sort: Sort;
}

export interface CompareState extends Scope {
  /**
   * The two rows of the head-to-head, as `[arch:]language/compiler/interpreter/mode`.
   * `null` — the site pairs the fastest with the fastest of another language.
   *
   * The ISA belongs to the *row's* address, not to the page's, because the two sides
   * may come from two campaigns: `?a=x86_64:c/gcc/-/strict&b=aarch64:c/gcc/-/strict`
   * is a legitimate thing to ask for and an alarming thing to be handed — the page
   * says so when it happens. Without a prefix a side falls back to `arch`, so every
   * link written before this existed still opens the pair it named.
   *
   * A comparison *is* a claim, and it is the sharpest one this site makes. It gets
   * a URL like every other view: a head-to-head somebody cannot link to is a
   * head-to-head nobody can check.
   */
  left: string | null;
  right: string | null;
}

/** One side of the pair, as the query string spells it: an ISA, and a row on it. */
export interface SideRef {
  /** `null` — the side named none, so it belongs to whichever campaign is in scope. */
  arch: string | null;
  /** `language/compiler/interpreter/mode`, or `null` when the side names no row. */
  key: string | null;
}

/**
 * `x86_64:c/gcc/-/strict` — the ISA, then the row.
 *
 * Validated, never trusted, like every other thing the query string says: an ISA this
 * build never published is dropped by whoever holds the campaigns, and so is a row.
 */
export function readSide(raw: string | null): SideRef {
  if (raw === null) {
    return { arch: null, key: null };
  }
  const colon = raw.indexOf(":");
  if (colon < 0) {
    return { arch: null, key: raw };
  }
  return { arch: raw.slice(0, colon), key: raw.slice(colon + 1) };
}

export function writeSide(arch: string, key: string): string {
  return `${arch}:${key}`;
}

/** Fastest first, on the statistic the report headlines. The same default as `report.md`. */
const DEFAULT_SORT: Sort = { key: "run", descending: false };

export const NO_FILTERS: Filters = {
  language: null,
  compiler: null,
  interpreter: null,
  search: "",
  modes: MODES,
};

function params(): URLSearchParams {
  return new URLSearchParams(window.location.search);
}

function readScope(query: URLSearchParams): Scope {
  return {
    arch: query.get("arch"),
    algo: query.get("algo"),
    includeWarmup: query.get("warmup") === "1",
  };
}

function writeScope(query: URLSearchParams, scope: Scope): void {
  if (scope.arch !== null) {
    query.set("arch", scope.arch);
  }
  if (scope.algo !== null) {
    query.set("algo", scope.algo);
  }
  if (scope.includeWarmup) {
    query.set("warmup", "1");
  }
}

export function readResults(): ResultsState {
  const query = params();

  const modes = (query.get("mode") ?? "")
    .split(",")
    .map((raw) => fpModeSchema.safeParse(raw))
    .flatMap((parsed) => (parsed.success ? [parsed.data] : []));

  const sortKey = sortKeySchema.safeParse(query.get("sort"));

  return {
    ...readScope(query),
    filters: {
      language: query.get("language"),
      compiler: query.get("compiler"),
      interpreter: query.get("interpreter"),
      search: query.get("q") ?? "",
      // Kept in the canonical order, never in the order they were typed: the mode
      // owns its colour, and a series that reorders itself repaints the chart.
      modes: modes.length > 0 ? MODES.filter((mode) => modes.includes(mode)) : MODES,
    },
    sort: sortKey.success
      ? { key: sortKey.data as SortKey, descending: query.get("desc") === "1" }
      : DEFAULT_SORT,
  };
}

export function writeResults(state: ResultsState): void {
  const query = new URLSearchParams();
  writeScope(query, state);

  const { filters } = state;
  if (filters.language !== null) {
    query.set("language", filters.language);
  }
  if (filters.compiler !== null) {
    query.set("compiler", filters.compiler);
  }
  if (filters.interpreter !== null) {
    query.set("interpreter", filters.interpreter);
  }
  if (filters.search !== "") {
    query.set("q", filters.search);
  }
  if (filters.modes.length !== MODES.length) {
    query.set("mode", filters.modes.join(","));
  }
  if (state.sort.key !== DEFAULT_SORT.key || state.sort.descending) {
    query.set("sort", state.sort.key);
    if (state.sort.descending) {
      query.set("desc", "1");
    }
  }

  replace(query);
}

export function readCompare(): CompareState {
  const query = params();
  return {
    ...readScope(query),
    left: query.get("a"),
    right: query.get("b"),
  };
}

export function writeCompare(state: CompareState): void {
  const query = new URLSearchParams();
  writeScope(query, state);
  if (state.left !== null) {
    query.set("a", state.left);
  }
  if (state.right !== null) {
    query.set("b", state.right);
  }
  replace(query);
}

function replace(query: URLSearchParams): void {
  const search = query.toString();
  // `replaceState`: a sort click is not a navigation, and burying the back button
  // under twelve of them helps nobody.
  window.history.replaceState(
    null,
    "",
    search === "" ? window.location.pathname : `${window.location.pathname}?${search}`,
  );
}

/**
 * The link from the results to the head-to-head, carrying the scope.
 *
 * The filters do not travel: they narrow a table, and a pair is not a table. But
 * the campaign and the algorithm do — an absolute timing never crosses an ISA, and
 * a "Compare" link that quietly switched architecture would be inviting exactly the
 * comparison `METHODOLOGY.md#the-isa-rule` forbids.
 */
export function compareHref(scope: Scope, left?: string, right?: string): string {
  const query = new URLSearchParams();
  writeScope(query, scope);
  if (left !== undefined) {
    query.set("a", left);
  }
  if (right !== undefined) {
    query.set("b", right);
  }
  const search = query.toString();
  const base = `${import.meta.env.BASE_URL}compare/`;
  return search === "" ? base : `${base}?${search}`;
}
