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
import { type FpMode, fpModeSchema } from "./analysis";
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
  algo: string | null;
  language: string | null;
  search: string;
  modes: FpMode[];
  includeWarmup: boolean;
  sort: Sort;
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
    algo: params.get("algo"),
    language: params.get("language"),
    search: params.get("q") ?? "",
    modes: modes.length > 0 ? MODES.filter((mode) => modes.includes(mode)) : MODES,
    includeWarmup: params.get("warmup") === "1",
    sort: sortKey.success
      ? { key: sortKey.data as SortKey, descending: params.get("desc") === "1" }
      : DEFAULT_SORT,
  };
}

export function writeUrl(state: UrlState): void {
  const params = new URLSearchParams();
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

  const query = params.toString();
  // `replaceState`: a sort click is not a navigation, and burying the back button
  // under twelve of them helps nobody.
  window.history.replaceState(
    null,
    "",
    query === "" ? window.location.pathname : `${window.location.pathname}?${query}`,
  );
}
