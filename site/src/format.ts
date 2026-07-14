// Units belong to the value, not to the template: `n/a ms` is nonsense.
//
// The same rules as `src/report.rs` formats the Markdown table with — the numbers
// arrive already computed, and all that is left is how to spell them.

import type { Summary } from "./analysis";

export const NOT_AVAILABLE = "n/a";

export function milliseconds(nanoseconds: number | null | undefined): string {
  if (nanoseconds === null || nanoseconds === undefined) {
    return NOT_AVAILABLE;
  }
  return `${(nanoseconds / 1e6).toFixed(1)} ms`;
}

export function seconds(microseconds: number | null | undefined): string {
  if (microseconds === null || microseconds === undefined) {
    return NOT_AVAILABLE;
  }
  return `${(microseconds / 1e6).toFixed(2)} s`;
}

export function bytes(value: number | null): string {
  if (value === null) {
    return NOT_AVAILABLE;
  }
  return value < 1024 ? `${value} B` : `${(value / 1024).toFixed(1)} KiB`;
}

/**
 * Below three samples the median absolute deviation is structurally zero — the
 * lower median of `[0, d]` is `0` — so reporting it would claim a precision the
 * campaign never had.
 */
export function dispersion(summary: Summary | null): string {
  if (summary === null) {
    return NOT_AVAILABLE;
  }
  return summary.n >= 3 ? `${summary.mad_pct.toFixed(2)}%` : `${NOT_AVAILABLE} (n=${summary.n})`;
}

/**
 * A size, scaled to the unit a human would have chosen: `812 B`, `70.8 KiB`, `3.6 MiB`.
 *
 * For the head-to-head, and only for it. The wire says `bytes` for a binary and for a
 * container's peak memory alike — they are the same unit and three orders of
 * magnitude apart — and a pair of rows is two numbers side by side, where scaling
 * each to fit is a kindness. A *column* is not: a column where one row reads
 * `900 KiB` and the next `3.6 MiB` cannot be scanned, which is why the table pins
 * Memory to MiB and Binary to KiB and neither of them uses this.
 */
export function size(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return NOT_AVAILABLE;
  }
  if (value < 1024) {
    return `${value} B`;
  }
  const kib = value / 1024;
  return kib < 1024 ? `${kib.toFixed(1)} KiB` : `${(kib / 1024).toFixed(1)} MiB`;
}

/**
 * A whole container's memory, which is megabytes and not kilobytes: a JVM's peak in
 * KiB is a six-digit number nobody reads at a glance. The same spelling as
 * the results table's Memory column.
 */
export function mebibytes(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return NOT_AVAILABLE;
  }
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}

/**
 * Cores kept busy, against the cores the kernel was given: `7.8 / 8`.
 *
 * The denominator is what makes the number readable — `7.8` alone says nothing until
 * you know whether eight threads were on offer or two. The harness measures both; the
 * site spells the pair. And the numerator is the **median**, the one statistic on the
 * row that is not a minimum: contention pushes a spinning thread's CPU clock and the
 * compute clock in both directions, so there is no one-sided noise to argue from.
 */
export function cores(summary: Summary | null, cpu: number): string {
  if (summary === null) {
    return NOT_AVAILABLE;
  }
  return `${(summary.median / 1e3).toFixed(1)} / ${cpu}`;
}

/** An absent half of the triple is a fact about the backend, so it is rendered. */
export function optional(value: string | null): string {
  return value ?? NOT_AVAILABLE;
}

/**
 * A ratio against the fastest backend of the same table: `1.00×`, `13.4×`.
 *
 * The only cross-backend number the site publishes. Absolute timings are a
 * property of the machine; a ratio within one campaign, on one architecture, is a
 * property of the backends. See `site/src/content/methodology.md#flags-and-the-architecture-baseline`.
 */
export function ratio(value: number, reference: number): string {
  if (reference === 0) {
    return NOT_AVAILABLE;
  }
  return times(value / reference);
}

/**
 * A ratio, spelled: `1.00×`, `13.4×`.
 *
 * Takes the number already computed rather than the two it came from — the
 * head-to-head's ratios are `src/compare.rs`'s, and a division redone here would
 * be a second definition of the site's one cross-backend claim.
 */
export function times(value: number | null): string {
  if (value === null) {
    return NOT_AVAILABLE;
  }
  return value < 10 ? `${value.toFixed(2)}×` : `${value.toFixed(1)}×`;
}

/** A percentage the harness computed — a gap, or the dispersion it has to clear. */
export function percent(value: number | null): string {
  if (value === null) {
    return NOT_AVAILABLE;
  }
  return `${value.toFixed(1)}%`;
}
