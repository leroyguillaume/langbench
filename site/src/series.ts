// Color follows the entity, never its rank.
//
// A mode owns its slot for the life of the site: `strict` is blue whether it is
// the only mode on screen or the third of three, and a filter that removes `fma`
// does not repaint `fast`. Assigned in the validated order, never cycled.

import type { FpMode } from "./analysis";
import type { Series } from "./components/BarChart";

export const MODE_COLOR: Record<FpMode, string> = {
  strict: "--series-1",
  fma: "--series-2",
  fast: "--series-3",
};

export const MODES: FpMode[] = ["strict", "fma", "fast"];

export function modeSeries(modes: FpMode[]): Series[] {
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
