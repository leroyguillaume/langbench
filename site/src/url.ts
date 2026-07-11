// The view lives in the URL.
//
// A filtered, sorted table is a claim about the data, and a claim you cannot link
// to is a claim nobody can check. So every knob round-trips through the query
// string: the address bar always describes what is on screen.
//
// The query string is an I/O boundary like any other — it is validated, never
// trusted. `?mode=strict,rubbish` narrows to `strict`; `?sort=drop_table` falls
// back to the default rather than reaching the sort with a key it has no column for.

import { z } from "zod";
import { type FpMode, fpModeSchema, type Row } from "./analysis";
import { rowKey } from "./components/Compare";
import type { Sort, SortKey } from "./components/ResultsTable";
import { MODES } from "./series";

const sortKeySchema = z.enum([
  "backend",
  "mode",
  "run",
  "dispersion",
  "compute",
  "startup",
  "cpu",
  "build",
  "binary",
  "text",
]);

export interface UrlState {
  /** The ISA whose campaign is on screen. `null` — whichever one sorts first. */
  arch: string | null;
  algo: string | null;
  language: string | null;
  search: string;
  modes: FpMode[];
  includeWarmup: boolean;
  sort: Sort;
  /**
   * The two rows of the head-to-head, `backend:mode`. `null` — the site pairs the
   * fastest with its runner-up.
   *
   * A comparison *is* a claim, and it is the sharpest one this site makes. It gets
   * a URL like every other view: a head-to-head somebody cannot link to is a
   * head-to-head nobody can check.
   */
  compareLeft: Row | null;
  compareRight: Row | null;
}

/**
 * A row as the query string spells it: `c-gcc:strict`.
 *
 * Validated, never trusted — the mode has to be one of the three, and a backend
 * this campaign never measured is dropped later, by whoever holds the aggregates.
 */
function readRow(raw: string | null): Row | null {
  if (raw === null) {
    return null;
  }
  const [backend, mode] = raw.split(":");
  const parsed = fpModeSchema.safeParse(mode);
  if (backend === undefined || backend === "" || !parsed.success) {
    return null;
  }
  return { backend, mode: parsed.data };
}

/** Fastest first, on the statistic the report headlines. The same default as `report.md`. */
const DEFAULT_SORT: Sort = { key: "run", descending: false };

export function readUrl(): UrlState {
  const params = new URLSearchParams(window.location.search);

  const modes = (params.get("mode") ?? "")
    .split(",")
    .map((raw) => fpModeSchema.safeParse(raw))
    .flatMap((parsed) => (parsed.success ? [parsed.data] : []));

  const sortKey = sortKeySchema.safeParse(params.get("sort"));

  return {
    arch: params.get("arch"),
    algo: params.get("algo"),
    language: params.get("language"),
    search: params.get("q") ?? "",
    modes: modes.length > 0 ? MODES.filter((mode) => modes.includes(mode)) : MODES,
    includeWarmup: params.get("warmup") === "1",
    sort: sortKey.success
      ? { key: sortKey.data as SortKey, descending: params.get("desc") === "1" }
      : DEFAULT_SORT,
    compareLeft: readRow(params.get("a")),
    compareRight: readRow(params.get("b")),
  };
}

export function writeUrl(state: UrlState): void {
  const params = new URLSearchParams();
  if (state.arch !== null) {
    params.set("arch", state.arch);
  }
  if (state.algo !== null) {
    params.set("algo", state.algo);
  }
  if (state.language !== null) {
    params.set("language", state.language);
  }
  if (state.search !== "") {
    params.set("q", state.search);
  }
  if (state.modes.length !== MODES.length) {
    params.set("mode", state.modes.join(","));
  }
  if (state.includeWarmup) {
    params.set("warmup", "1");
  }
  if (state.sort.key !== DEFAULT_SORT.key || state.sort.descending) {
    params.set("sort", state.sort.key);
    if (state.sort.descending) {
      params.set("desc", "1");
    }
  }
  if (state.compareLeft !== null) {
    params.set("a", rowKey(state.compareLeft));
  }
  if (state.compareRight !== null) {
    params.set("b", rowKey(state.compareRight));
  }

  const query = params.toString();
  // `replaceState`: a sort click is not a navigation, and burying the back button
  // under twelve of them helps nobody.
  window.history.replaceState(
    null,
    "",
    query === "" ? window.location.pathname : `${window.location.pathname}?${query}`,
  );
}
