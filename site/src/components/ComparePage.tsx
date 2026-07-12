// Two languages, side by side — and the one thing a table of two rows never says
// out loud: whether the gap between them is a difference.
//
// You pick a language first, and then the toolchain that ran it, because that is
// the question a reader actually arrives with: *is Rust faster than C here?* — and
// then, immediately, *which Rust, and compiled by what?* Two languages with one
// compiler each is a comparison; "java-native vs c-gcc" is a pair of slugs, and the
// first thing you have to do with a slug is decode it.
//
// Every number on this page was computed by `src/compare.rs`: the ratio, the gap,
// the dispersion the gap has to clear, and the verdict when it does not. Nothing
// here divides, compares or rounds a measurement — it picks two rows, spells what
// the harness said about them, and colours the answer. A tie is not a formatting
// choice; it is the campaign refusing to defend a claim it cannot afford.
// See `METHODOLOGY.md#a-difference-smaller-than-the-dispersion-is-not-a-difference`.

import { useEffect, useMemo, useState } from "react";
import type { Aggregate, Comparison, FpMode, LoadedCampaign, Metric } from "../analysis";
import { compare } from "../analysis";
import { useCampaigns } from "../campaigns";
import { bytes, milliseconds, NOT_AVAILABLE, optional, percent, seconds, times } from "../format";
import { findByKey, type Identity, identityKey, label, toolchain, wasmRow } from "../identity";
import { logger } from "../logger";
import { MODE_COLOR, MODES } from "../series";
import { type CompareState, readCompare, writeCompare } from "../url";
import { Warmup, WarmupBanner } from "./Warmup";

export function ComparePage() {
  const [state, setState] = useState<CompareState>(readCompare);
  const { campaigns, error, pending } = useCampaigns(state.includeWarmup);

  useEffect(() => writeCompare(state), [state]);

  if (error !== null) {
    return (
      <main className="page">
        <div className="warnings">
          <h2>This campaign could not be read</h2>
          <p>{error}</p>
        </div>
      </main>
    );
  }
  // Only when there is nothing to show at all. A *re*-aggregation keeps the previous
  // numbers on screen and dims them: tearing the page down and putting it back
  // collapses the document, and the browser takes the reader's scroll position with
  // it.
  if (campaigns === null) {
    return <p className="status">Reading the campaigns…</p>;
  }

  const loaded = campaigns.find((entry) => entry.analysis.arch === state.arch) ?? campaigns[0];
  if (loaded === undefined) {
    return (
      <main className="page">
        <div className="warnings">
          <h2>This build publishes no campaign</h2>
        </div>
      </main>
    );
  }

  return (
    <Head2Head
      loaded={loaded}
      campaigns={campaigns}
      state={state}
      setState={setState}
      pending={pending}
    />
  );
}

interface Props {
  loaded: LoadedCampaign;
  campaigns: LoadedCampaign[];
  state: CompareState;
  setState: (state: CompareState) => void;
  /** The harness is re-aggregating; these numbers are the previous ones. */
  pending: boolean;
}

function Head2Head({ loaded, campaigns, state, setState, pending }: Props) {
  const { analysis, ndjson } = loaded;
  const algo = analysis.algos.find((entry) => entry.algo === state.algo) ?? analysis.algos[0];
  const aggregates = useMemo(
    () => (algo?.aggregates ?? []).filter((row) => row.run_wall !== null),
    [algo],
  );

  // The default pair is the fastest row and the fastest row of *another language*.
  // Two rows of the same language, one compiled by gcc and one by clang, is a fine
  // question — but it is not the one this page opens with, and a reader who wanted
  // it can pick it in two clicks. The aggregates arrive fastest first: the harness
  // sorted them, on the statistic the report headlines.
  const [left, right] = useMemo(() => {
    const first = findByKey(aggregates, state.left) ?? aggregates[0] ?? null;
    const fallback =
      aggregates.find((row) => row.language !== first?.language) ??
      aggregates.find((row) => row !== first) ??
      null;
    return [first, findByKey(aggregates, state.right) ?? fallback] as const;
  }, [aggregates, state.left, state.right]);

  // The comparison is the harness's, computed from the raw campaign — the same
  // file, the same code, the same min-of-N as every other number on this site. The
  // site never decides whether a gap is a difference.
  const comparison: { value: Comparison | null; error: string | null } = useMemo(() => {
    if (left === null || right === null || algo === undefined) {
      return { value: null, error: null };
    }
    try {
      const value = compare(
        ndjson,
        { include_warmup: state.includeWarmup },
        { algo: algo.algo, left: wasmRow(left), right: wasmRow(right) },
      );
      return { value, error: null };
    } catch (cause: unknown) {
      // A pair the campaign cannot honour is a broken card, never a broken page.
      const message = cause instanceof Error ? cause.message : String(cause);
      logger.error("compare.failed", { message });
      return { value: null, error: message };
    }
  }, [ndjson, algo, left, right, state.includeWarmup]);

  if (left === null || right === null) {
    return (
      <main className="page">
        <header className="masthead">
          <h1>Head to head</h1>
        </header>
        <section className="card">
          <p>
            This campaign measured fewer than two rows on{" "}
            <code>{algo?.algo ?? "this algorithm"}</code>: there is no pair to compare.
          </p>
        </section>
      </main>
    );
  }

  const pick = (side: "left" | "right", row: Aggregate) =>
    setState({ ...state, [side]: identityKey(row) });

  const swap = () => setState({ ...state, left: state.right, right: state.left });

  const run = comparison.value?.metrics.find((metric) => metric.key === "run") ?? null;

  return (
    <main className={pending ? "page recomputing" : "page"} aria-busy={pending}>
      <header className="masthead">
        <h1>Head to head</h1>
        <p>
          Two rows of the <strong>{analysis.arch}</strong> campaign, and the verdict the campaign
          can defend. A gap smaller than the dispersion the two rows carry is{" "}
          <strong>not a difference</strong> — it is the same number, measured twice, on a machine
          that was busy. The harness decides that, not this page.
        </p>
      </header>

      <div className="filters">
        {campaigns.length > 1 && (
          <label className="filter">
            <span>ISA</span>
            <select
              value={analysis.arch}
              onChange={(event) => setState({ ...state, arch: event.target.value })}
            >
              {campaigns.map((entry) => (
                <option key={entry.analysis.arch} value={entry.analysis.arch}>
                  {entry.analysis.arch}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="filter">
          <span>Algorithm</span>
          <select
            value={algo?.algo ?? ""}
            onChange={(event) => setState({ ...state, algo: event.target.value })}
          >
            {analysis.algos.map((entry) => (
              <option key={entry.algo} value={entry.algo}>
                {entry.algo}
              </option>
            ))}
          </select>
        </label>
      </div>

      <Warmup
        rounds={analysis.campaign.warmup_rounds}
        includeWarmup={state.includeWarmup}
        onChange={(includeWarmup) => setState({ ...state, includeWarmup })}
        compact
      />

      {state.includeWarmup && analysis.campaign.warmup_rounds > 0 && (
        <WarmupBanner rounds={analysis.campaign.warmup_rounds} />
      )}

      <div className="compare-pick">
        <Picker side="left" title="A" aggregates={aggregates} selected={left} onPick={pick} />
        <button type="button" className="compare-swap" onClick={swap} aria-label="swap A and B">
          ⇄
        </button>
        <Picker side="right" title="B" aggregates={aggregates} selected={right} onPick={pick} />
      </div>

      {comparison.error !== null && <p className="compare-error">{comparison.error}</p>}

      {comparison.value !== null && (
        <>
          <Headline comparison={comparison.value} run={run} />
          <Checksums comparison={comparison.value} />

          <section className="card">
            <h2>Every metric</h2>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th className="text">Metric</th>
                    <th className="numeric">
                      <Name identity={left} />
                    </th>
                    <th className="numeric">
                      <Name identity={right} />
                    </th>
                    <th className="numeric">B ÷ A</th>
                    <th className="text">Verdict</th>
                  </tr>
                </thead>
                <tbody>
                  {comparison.value.metrics.map((metric) => (
                    <tr key={metric.key}>
                      <td className="text">{metric.label}</td>
                      <td className={cell(metric, "left")}>{value(metric, "left")}</td>
                      <td className={cell(metric, "right")}>{value(metric, "right")}</td>
                      <td className="numeric">{times(metric.ratio)}</td>
                      <td className="text">
                        <Verdict metric={metric} left={left} right={right} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}
    </main>
  );
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

/** The winning cell is marked — with a weight and a background, never with colour alone. */
function cell(metric: Metric, side: "left" | "right"): string {
  return metric.verdict === side ? "numeric better" : "numeric";
}

function Name({ identity }: { identity: Identity }) {
  return (
    <span className="mode-tag">
      <span className="mode-dot" style={{ background: `var(${MODE_COLOR[identity.mode]})` }} />
      {label(identity)} · {identity.mode}
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
      On the run, <strong>{label(winner)}</strong> in <strong>{winner.mode}</strong> is faster by{" "}
      <strong>{percent(run.gap_pct)}</strong>
      {run.noise_pct !== null && <>, against a dispersion of {percent(run.noise_pct)}</>}. A ratio
      within one campaign, on one ISA — the only cross-implementation claim this project publishes.
    </p>
  );
}

/**
 * Whether the two rows computed the same thing. Nothing beside it means anything
 * until this does: an implementation that is fast because it got the wrong answer is
 * not fast, it is wrong.
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
        checksum column on the results page prices it.
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

function Verdict({ metric, left, right }: { metric: Metric; left: Identity; right: Identity }) {
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
      const winner = metric.verdict === "left" ? left : right;
      return (
        <span>
          <strong>{label(winner)}</strong> · {winner.mode}, by {percent(metric.gap_pct)}
        </span>
      );
    }
  }
}

interface PickerProps {
  side: "left" | "right";
  title: string;
  aggregates: Aggregate[];
  selected: Aggregate;
  onPick: (side: "left" | "right", row: Aggregate) => void;
}

/**
 * Language, then toolchain, then mode — three questions, in the order a reader asks
 * them.
 *
 * Each list is built from the rows this campaign actually measured, and each is
 * scoped by the answer above it: pick `python` and the toolchains are `cpython`,
 * `cython + cpython`, `pypy`, because those are the ones that ran. Changing a
 * language keeps the mode if the new language has it and falls back to its first row
 * if it does not — a selector that can land on a row the campaign never measured is
 * a selector that can produce a blank page.
 */
function Picker({ side, title, aggregates, selected, onPick }: PickerProps) {
  const languages = [...new Set(aggregates.map((row) => row.language))].sort();
  const sameLanguage = aggregates.filter((row) => row.language === selected.language);

  // One entry per toolchain, not one per row: a toolchain measured in three modes is
  // one toolchain, and the mode is the next question, not part of this one.
  const toolchains = [...new Map(sameLanguage.map((row) => [toolchain(row), row])).values()];
  const modes = MODES.filter((mode) =>
    sameLanguage.some((row) => toolchain(row) === toolchain(selected) && row.mode === mode),
  );

  /** The row closest to what is selected, among a set the reader just narrowed to. */
  const nearest = (candidates: Aggregate[]): Aggregate | undefined =>
    candidates.find(
      (row) => toolchain(row) === toolchain(selected) && row.mode === selected.mode,
    ) ??
    candidates.find((row) => row.mode === selected.mode) ??
    candidates[0];

  const setLanguage = (language: string) => {
    const row = nearest(aggregates.filter((candidate) => candidate.language === language));
    if (row !== undefined) {
      onPick(side, row);
    }
  };

  const setToolchain = (chain: string) => {
    const row = nearest(sameLanguage.filter((candidate) => toolchain(candidate) === chain));
    if (row !== undefined) {
      onPick(side, row);
    }
  };

  const setMode = (mode: string) => {
    const row = sameLanguage.find(
      (candidate) => toolchain(candidate) === toolchain(selected) && candidate.mode === mode,
    );
    if (row !== undefined) {
      onPick(side, row);
    }
  };

  return (
    <div className="compare-side">
      <div className="compare-side-title">{title}</div>

      <label className="filter">
        <span>Language</span>
        <select value={selected.language} onChange={(event) => setLanguage(event.target.value)}>
          {languages.map((language) => (
            <option key={language} value={language}>
              {language}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>Compiler · interpreter</span>
        <select value={toolchain(selected)} onChange={(event) => setToolchain(event.target.value)}>
          {toolchains.map((row) => (
            <option key={toolchain(row)} value={toolchain(row)}>
              {toolchain(row)}
            </option>
          ))}
        </select>
      </label>

      <label className="filter">
        <span>FP mode</span>
        <select value={selected.mode} onChange={(event) => setMode(event.target.value as FpMode)}>
          {modes.map((mode) => (
            <option key={mode} value={mode}>
              {mode}
            </option>
          ))}
        </select>
      </label>

      <dl className="compare-side-triple">
        <dt>language</dt>
        <dd>{selected.language}</dd>
        <dt>compiler</dt>
        <dd>{optional(selected.compiler)}</dd>
        <dt>interpreter</dt>
        <dd>{optional(selected.interpreter)}</dd>
      </dl>
    </div>
  );
}
