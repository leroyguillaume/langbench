// Two backends, side by side — and the one thing a table of two rows never says
// out loud: whether the gap between them is a difference.
//
// Every number on this panel was computed by `src/compare.rs`: the ratio, the gap,
// the dispersion the gap has to clear, and the verdict when it does not. Nothing
// here divides, compares or rounds a measurement — it picks two rows, spells what
// the harness said about them, and colours the answer. A tie is not a formatting
// choice; it is the campaign refusing to defend a claim it cannot afford.
// See `METHODOLOGY.md#a-difference-smaller-than-the-dispersion-is-not-a-difference`.

import type { Aggregate, Comparison, FpMode, Metric, Row, Selection, Side } from "../analysis";
import { bytes, milliseconds, NOT_AVAILABLE, optional, percent, seconds, times } from "../format";
import { MODE_COLOR } from "../series";

/** How a row is named in the URL and in a `<select>`: `c-gcc:strict`. */
export function rowKey(row: Row): string {
  return `${row.backend}:${row.mode}`;
}

/**
 * The pair to put head to head: the reader's, when the campaign has both rows,
 * and otherwise the two the report ranks first.
 *
 * A row the reader linked to and this campaign never measured is dropped rather
 * than passed on — the WASM would refuse it, and refusing a whole page over a
 * stale bookmark helps nobody. `null` below two measured rows: a comparison needs
 * something to compare.
 */
export function resolve(
  aggregates: Aggregate[],
  algo: string,
  left: Row | null,
  right: Row | null,
): Selection | null {
  const known = (row: Row | null): Row | null =>
    aggregates.some(
      (candidate) => candidate.backend === row?.backend && candidate.mode === row.mode,
    )
      ? row
      : null;

  // The aggregates arrive fastest first — the harness sorted them, on the same
  // statistic the report headlines. So the default pair is the winner and its
  // runner-up: the comparison a reader was about to make by hand anyway.
  const fallback = aggregates.filter((row) => row.run_wall !== null);
  const first = known(left) ?? fallback[0] ?? null;
  const second =
    known(right) ?? fallback.find((row) => rowKey(row) !== (first && rowKey(first))) ?? null;

  if (first === null || second === null) {
    return null;
  }
  return {
    algo,
    left: { backend: first.backend, mode: first.mode },
    right: { backend: second.backend, mode: second.mode },
  };
}

/** A value in whatever the harness measured it in. The unit travels with the number. */
function value(metric: Metric, side: "left" | "right"): string {
  const raw = metric[side];
  switch (metric.unit) {
    case "nanoseconds":
      return milliseconds(raw);
    case "microseconds":
      return seconds(raw);
    case "bytes":
      return bytes(raw);
  }
}

interface Props {
  /** Every row of the algorithm on screen — the filters above do not narrow the pair. */
  aggregates: Aggregate[];
  selection: Selection | null;
  /** `null` when the WASM refused the pair; `error` says why. */
  comparison: Comparison | null;
  error: string | null;
  onSelect: (selection: Selection) => void;
}

export function Compare({ aggregates, selection, comparison, error, onSelect }: Props) {
  if (selection === null) {
    return (
      <section className="card">
        <h2>Head to head</h2>
        <p>This campaign measured fewer than two rows: there is no pair to compare.</p>
      </section>
    );
  }

  const pick = (side: "left" | "right", key: string) => {
    const [backend, mode] = key.split(":");
    if (backend === undefined || mode === undefined) {
      return;
    }
    onSelect({ ...selection, [side]: { backend, mode: mode as FpMode } });
  };

  const swap = () => onSelect({ ...selection, left: selection.right, right: selection.left });

  const run = comparison?.metrics.find((metric) => metric.key === "run") ?? null;

  return (
    <section className="card">
      <h2>Head to head</h2>
      <p>
        Two rows of this campaign, and the verdict the campaign can defend. A gap smaller than the
        dispersion the two rows carry is <strong>not a difference</strong> — it is the same number,
        measured twice, on a machine that was busy. The harness decides that, not this page.
      </p>

      <div className="compare-pick">
        <Picker
          side="left"
          label="Backend A"
          aggregates={aggregates}
          selected={selection.left}
          onPick={pick}
        />
        <button type="button" className="compare-swap" onClick={swap} aria-label="swap A and B">
          ⇄
        </button>
        <Picker
          side="right"
          label="Backend B"
          aggregates={aggregates}
          selected={selection.right}
          onPick={pick}
        />
      </div>

      {error !== null && <p className="compare-error">{error}</p>}

      {comparison !== null && (
        <>
          <Headline comparison={comparison} run={run} />
          <Checksums comparison={comparison} />

          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th className="text">Metric</th>
                  <th className="numeric">
                    <Name side={comparison.left} />
                  </th>
                  <th className="numeric">
                    <Name side={comparison.right} />
                  </th>
                  <th className="numeric">B ÷ A</th>
                  <th className="text">Verdict</th>
                </tr>
              </thead>
              <tbody>
                {comparison.metrics.map((metric) => (
                  <tr key={metric.key}>
                    <td className="text">{metric.label}</td>
                    <td className={cell(metric, "left")}>{value(metric, "left")}</td>
                    <td className={cell(metric, "right")}>{value(metric, "right")}</td>
                    <td className="numeric">{times(metric.ratio)}</td>
                    <td className="text">
                      <Verdict metric={metric} comparison={comparison} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
}

/** The winning cell is marked — with a weight and a background, never with colour alone. */
function cell(metric: Metric, side: "left" | "right"): string {
  return metric.verdict === side ? "numeric better" : "numeric";
}

function Name({ side }: { side: Side }) {
  return (
    <span
      className="mode-tag"
      title={`${side.language} · ${optional(side.compiler)} · ${optional(side.interpreter)}`}
    >
      <span className="mode-dot" style={{ background: `var(${MODE_COLOR[side.mode]})` }} />
      {side.backend} · {side.mode}
    </span>
  );
}

/**
 * The sentence a reader came for, on the metric the report headlines: the external
 * wall-clock. Spelled out rather than left to be read off a ratio column, because
 * "1.03×" and "indistinguishable" look the same to somebody in a hurry — and only
 * one of them is a result.
 */
function Headline({ comparison, run }: { comparison: Comparison; run: Metric | null }) {
  if (run === null || run.verdict === "unmeasured") {
    return (
      <p className="compare-headline">
        This campaign has no wall-clock for one of these two rows, so it ranks neither.
      </p>
    );
  }
  if (run.verdict === "tie") {
    return (
      <p className="compare-headline tie">
        On the run, these two are <strong>indistinguishable</strong>. The gap is{" "}
        {percent(run.gap_pct)} and the noisier of the two rows wobbles by {percent(run.noise_pct)} —
        this campaign cannot tell them apart, whichever minimum came out lower.
      </p>
    );
  }
  const winner = run.verdict === "left" ? comparison.left : comparison.right;
  return (
    <p className="compare-headline">
      On the run, <strong>{winner.backend}</strong> in <strong>{winner.mode}</strong> is faster by{" "}
      <strong>{percent(run.gap_pct)}</strong>
      {run.noise_pct !== null && <> , against a dispersion of {percent(run.noise_pct)}</>}. A ratio
      within one campaign, on one ISA — the only cross-backend claim this project publishes.
    </p>
  );
}

/**
 * Whether the two rows computed the same thing. Nothing beside it means anything
 * until this does: a backend that is fast because it got the wrong answer is not
 * fast, it is wrong.
 */
function Checksums({ comparison }: { comparison: Comparison }) {
  const { checksums, left, right } = comparison;
  if (checksums.violates_strict_invariant) {
    return (
      <p className="compare-checksum warning">
        These two rows are both <code>strict</code> and their checksums disagree —{" "}
        <code>{checksums.left}</code> against <code>{checksums.right}</code>. In <code>strict</code>{" "}
        mode the checksum is bit-identical across every compiler and language; a divergence is a
        bug, never a rounding excuse. The harness aborts a campaign over it, so the timings above
        are not comparable and this file did not come from a clean run.
      </p>
    );
  }
  if (checksums.same === true) {
    return (
      <p className="compare-checksum">
        Both computed <code>{checksums.left ?? NOT_AVAILABLE}</code>: the same answer, to the bit.
      </p>
    );
  }
  if (checksums.same === false) {
    return (
      <p className="compare-checksum">
        Different answers — <code>{checksums.left}</code> against <code>{checksums.right}</code>.
        Expected: <code>{left.mode}</code> and <code>{right.mode}</code> do not promise the same
        arithmetic, and a relaxed mode's whole purpose is to sell precision for speed. The Δ
        checksum column in the table above prices it.
      </p>
    );
  }
  return (
    <p className="compare-checksum">
      One of these two rows reported no checksum, so the campaign cannot say they computed the same
      thing.
    </p>
  );
}

function Verdict({ metric, comparison }: { metric: Metric; comparison: Comparison }) {
  switch (metric.verdict) {
    case "unmeasured":
      return <span className="muted">one side has no such number</span>;
    case "tie":
      return metric.noise_pct === null || metric.gap_pct === 0 ? (
        <span className="tie">identical</span>
      ) : (
        <span className="tie" title={`the gap is ${percent(metric.gap_pct)}`}>
          indistinguishable (noise ±{percent(metric.noise_pct)})
        </span>
      );
    default: {
      const winner = metric.verdict === "left" ? comparison.left : comparison.right;
      return (
        <span>
          <strong>{winner.backend}</strong> · {winner.mode}, by {percent(metric.gap_pct)}
        </span>
      );
    }
  }
}

interface PickerProps {
  side: "left" | "right";
  label: string;
  aggregates: Aggregate[];
  selected: Row;
  onPick: (side: "left" | "right", key: string) => void;
}

function Picker({ side, label, aggregates, selected, onPick }: PickerProps) {
  return (
    <label className="filter">
      <span>{label}</span>
      <select value={rowKey(selected)} onChange={(event) => onPick(side, event.target.value)}>
        {aggregates.map((row) => (
          <option key={rowKey(row)} value={rowKey(row)}>
            {row.backend} · {row.mode}
          </option>
        ))}
      </select>
    </label>
  );
}
