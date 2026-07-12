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

/** A relaxed mode's distance from the strict reference, as an opaque token. */
export function delta(value: string | null): string {
  if (value === null) {
    return NOT_AVAILABLE;
  }
  // A string, and it stays one: the checksum is 64-bit and JavaScript's `+`
  // would round it. `startsWith("-")` is all the arithmetic we need.
  return value === "0" || value.startsWith("-") ? value : `+${value}`;
}

/**
 * A whole container's memory, which is megabytes and not kilobytes: a JVM's peak in
 * KiB is a six-digit number nobody reads at a glance. The same spelling as
 * `report.md`'s Memory column.
 */
export function mebibytes(value: number | null | undefined): string {
  if (value === null || value === undefined) {
    return NOT_AVAILABLE;
  }
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}

/**
 * Energy, in joules. `n/a` wherever the host exposes no counter — which is most
 * laptops and every virtualised runner, and is an absence rather than a zero.
 */
export function joules(microjoules: number | null | undefined): string {
  if (microjoules === null || microjoules === undefined) {
    return NOT_AVAILABLE;
  }
  return `${(microjoules / 1e6).toFixed(1)} J`;
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
 * property of the machine; a ratio within one campaign, on one ISA, is a
 * property of the backends. See `METHODOLOGY.md#the-isa-rule`.
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
