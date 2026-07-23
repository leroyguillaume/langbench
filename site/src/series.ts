// Color follows the entity, never its rank.
//
// A mode owns its slot for the life of the site: `baseline` is blue whether it is
// the only mode on screen or one of two, and a filter that removes `native` does not
// repaint it. Assigned in the validated order, never cycled.

import type { Mode } from "./analysis";
import type { Series } from "./components/BarChart";

export const MODE_COLOR: Record<Mode, string> = {
  baseline: "--series-1",
  native: "--series-2",
};

/**
 * The modes, in the order the harness declares them — the closed set, and the only
 * order a series is ever drawn in.
 *
 * It is not derived from a campaign: a mode is an enum of the harness, not a string a
 * manifest invented, and its colour has to be the same on every page of this site
 * whether or not the campaign on screen happens to carry a row in it. What *is*
 * derived from the campaign is which of them a reader is offered — see `FilterBar`.
 */
export const MODES: Mode[] = ["baseline", "native"];

export function modeSeries(modes: Mode[]): Series[] {
  return modes.map((mode) => ({
    key: mode,
    label: mode,
    color: MODE_COLOR[mode],
  }));
}

/** The two halves of the external wall-clock. Their own slots, not the modes'. */
export const WALL_SERIES: Series[] = [
  {
    key: "compute",
    label: "compute (the program's own clock)",
    color: "--series-4",
  },
  {
    key: "startup",
    label: "startup (container + runtime init)",
    color: "--series-5",
  },
];

/** Magnitude, one hue: a single-series bar is a sequential encoding, not a categorical one. */
export const SEQUENTIAL: Series[] = [{ key: "value", label: "value", color: "--sequential" }];
