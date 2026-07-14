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
// See `site/src/content/methodology.md#sampling-and-what-may-be-concluded`.

import { useEffect, useMemo, useState } from "react";
import type { Aggregate, Comparison, FpMode, LoadedCampaign, Metric } from "../analysis";
import { compare, compareAcross } from "../analysis";
import { useCampaigns } from "../campaigns";
import { milliseconds, NOT_AVAILABLE, optional, percent, seconds, size, times } from "../format";
import { findByKey, type Identity, identityKey, label, toolchain, wasmRow } from "../identity";
import { logger } from "../logger";
import { MODE_COLOR, MODES } from "../series";
import { type CompareState, readCompare, readSide, writeCompare, writeSide } from "../url";
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

  // A campaign is one machine measuring one workload: it takes both to name one.
  // See the same selection in `Results.tsx` — keying on the architecture alone would
  // pick whichever workload's campaign happened to be published first.
  const loaded =
    campaigns.find(
      (entry) =>
        entry.analysis.architecture === state.architecture &&
        entry.analysis.campaign.workload.id === state.workload,
    ) ??
    campaigns.find((entry) => entry.analysis.architecture === state.architecture) ??
    campaigns.find((entry) => entry.analysis.campaign.workload.id === state.workload) ??
    campaigns[0];
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
  const { analysis } = loaded;

  // **The workload of the campaign that actually loaded, never the one the query string
  // asked for.** They differ every time somebody arrives here without asking for one —
  // from the sidebar, say — and `state.workload` is then `null`. Every lookup below
  // names a campaign by `(workload, architecture)`, and a lookup for the workload
  // `null` matches nothing: the page rendered, because the campaign fell back to the
  // first published one, and then every control on it quietly did nothing, because they
  // all went looking for a campaign that does not exist. So the fallback is resolved
  // *once*, here, and what came out of it is what the rest of the page is about.
  const id = analysis.campaign.workload.id;

  // Each side names its own campaign. A side that names none belongs to the one in
  // scope — which is what every link written before this page could cross an architecture says.
  //
  // A side names an *architecture*, never a workload: the two sides of a head-to-head
  // are two backends doing the same work, and comparing a Mandelbrot to a JSON parse
  // would not be a slow backend, it would be a category error. So the workload comes
  // from the scope, and both sides are read out of a campaign that measured it.
  const campaignOn = (architecture: string | null): LoadedCampaign | undefined =>
    campaigns.find(
      (entry) =>
        entry.analysis.architecture === architecture && entry.analysis.campaign.workload.id === id,
    );

  const sideOf = (raw: string | null): LoadedCampaign =>
    campaignOn(readSide(raw).architecture) ?? loaded;

  const leftCampaign = sideOf(state.left);
  const rightCampaign = sideOf(state.right);

  const rowsOf = (campaign: LoadedCampaign): Aggregate[] => {
    const found = campaign.analysis.workloads.find((entry) => entry.workload === id);
    const chosen = found ?? campaign.analysis.workloads[0];
    return (chosen?.aggregates ?? []).filter((row) => row.run_wall !== null);
  };
  const leftRows = rowsOf(leftCampaign);
  const rightRows = rowsOf(rightCampaign);

  const workload = analysis.workloads.find((entry) => entry.workload === id);

  // The default pair is the fastest row and the fastest row of *another language*.
  // Two rows of the same language, one compiled by gcc and one by clang, is a fine
  // question — but it is not the one this page opens with, and a reader who wanted
  // it can pick it in two clicks. The aggregates arrive fastest first: the harness
  // sorted them, on the statistic the report headlines.
  const left = findByKey(leftRows, readSide(state.left).key) ?? leftRows[0] ?? null;
  const right =
    findByKey(rightRows, readSide(state.right).key) ??
    rightRows.find((row) => row.language !== left?.language) ??
    rightRows.find((row) => row !== left) ??
    null;

  // The comparison is the harness's — the same code, the same min-of-N as every other
  // number on this site. When the two rows come from two campaigns it is the harness
  // that reads both files, and the harness that flags the crossing: the site does not
  // divide a millisecond by another, here or anywhere.
  const comparison: { value: Comparison | null; error: string | null } = useMemo(() => {
    if (left === null || right === null || workload === undefined) {
      return { value: null, error: null };
    }
    const options = { include_warmup: state.includeWarmup };
    const selection = { workload: workload.workload, left: wasmRow(left), right: wasmRow(right) };
    try {
      const value =
        leftCampaign === rightCampaign
          ? compare(leftCampaign.ndjson, options, selection)
          : compareAcross(leftCampaign.ndjson, rightCampaign.ndjson, options, selection);
      return { value, error: null };
    } catch (cause: unknown) {
      // A pair the campaign cannot honour is a broken card, never a broken page.
      const message = cause instanceof Error ? cause.message : String(cause);
      logger.error("compare.failed", { message });
      return { value: null, error: message };
    }
  }, [leftCampaign, rightCampaign, workload, left, right, state.includeWarmup]);

  if (left === null || right === null) {
    return (
      <main className="page">
        <header className="masthead">
          <h1>Compare</h1>
        </header>
        <section className="card">
          <p>
            This campaign measured fewer than two rows on{" "}
            <code>{workload?.workload ?? "this workload"}</code>: there is no pair to compare.
          </p>
        </section>
      </main>
    );
  }

  const pick = (side: "left" | "right", architecture: string, row: Aggregate) =>
    setState({ ...state, [side]: writeSide(architecture, identityKey(row)) });

  // Moving a side to another architecture keeps the row it was on, if that campaign measured
  // it: "the same backend, on the other machine" is the question somebody switching
  // architecture is asking. Otherwise it lands on that campaign's fastest row.
  const moveTo = (side: "left" | "right", architecture: string) => {
    // The other machine, the *same* work: switching architecture asks "and how does
    // this backend do over there", which is only a question about one workload.
    const campaign = campaignOn(architecture);
    if (campaign === undefined) {
      return;
    }
    const current = side === "left" ? left : right;
    const rows = rowsOf(campaign);
    const row = findByKey(rows, identityKey(current)) ?? rows[0];
    if (row !== undefined) {
      pick(side, architecture, row);
    }
  };

  const swap = () => setState({ ...state, left: state.right, right: state.left });

  const view = comparison.value;
  const run = view?.metrics.find((metric) => metric.key === "run") ?? null;

  return (
    <main className={pending ? "page recomputing" : "page"} aria-busy={pending}>
      <header className="masthead">
        <h1>Compare</h1>
        <p>
          Two rows, and the verdict the campaign can defend. A gap smaller than the dispersion the
          two rows carry is <strong>not a difference</strong> — it is the same number, measured
          twice, on a machine that was busy. The harness decides that, not this page.
        </p>
      </header>

      <div className="filters">
        <label className="filter">
          <span>Workload</span>
          <select
            value={workload?.workload ?? ""}
            onChange={(event) => setState({ ...state, workload: event.target.value })}
          >
            {/* Every workload this build published, not the one on screen: a selector
                that only offers what is already selected is not a selector. */}
            {[...new Set(campaigns.map((entry) => entry.analysis.campaign.workload.id))].map(
              (id) => (
                <option key={id} value={id}>
                  {id}
                </option>
              ),
            )}
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
        <Picker
          side="left"
          title="A"
          aggregates={leftRows}
          selected={left}
          architecture={leftCampaign.analysis.architecture}
          architectures={campaigns.map((entry) => entry.analysis.architecture)}
          onPick={pick}
          onArch={moveTo}
        />
        <button type="button" className="compare-swap" onClick={swap} aria-label="swap A and B">
          ⇄
        </button>
        <Picker
          side="right"
          title="B"
          aggregates={rightRows}
          selected={right}
          architecture={rightCampaign.analysis.architecture}
          architectures={campaigns.map((entry) => entry.analysis.architecture)}
          onPick={pick}
          onArch={moveTo}
        />
      </div>

      {/* The two rows were measured on two machines. Everything below is still
          computed, and almost none of it means anything — said here rather than left
          for the reader to work out from a row of numbers that looks exactly like a
          valid one. */}
      {view?.cross_isa === true && (
        <CrossIsaWarning left={view.left.architecture} right={view.right.architecture} />
      )}

      {comparison.error !== null && <p className="compare-error">{comparison.error}</p>}

      {view !== null && (
        <>
          <Headline comparison={view} run={run} />
          <Checksums comparison={view} />

          <section className="card">
            <h2>Every metric</h2>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th className="text">Metric</th>
                    <th className="numeric">
                      <Name
                        identity={left}
                        {...(view.cross_isa ? { architecture: view.left.architecture } : {})}
                      />
                    </th>
                    <th className="numeric">
                      <Name
                        identity={right}
                        {...(view.cross_isa ? { architecture: view.right.architecture } : {})}
                      />
                    </th>
                    <th className="numeric">B ÷ A</th>
                    <th className="text">Verdict</th>
                  </tr>
                </thead>
                <tbody>
                  {view.metrics.map((metric) => (
                    <tr key={metric.key}>
                      <td className="text">{metric.label}</td>
                      <td className={cell(metric, "left")}>{value(metric, "left")}</td>
                      <td className={cell(metric, "right")}>{value(metric, "right")}</td>
                      <td className="numeric">{times(metric.ratio)}</td>
                      <td className="text">
                        <Verdict
                          metric={metric}
                          left={left}
                          right={right}
                          crossIsa={
                            view.cross_isa
                              ? {
                                  left: view.left.architecture,
                                  right: view.right.architecture,
                                }
                              : null
                          }
                        />
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
    // Scaled, not pinned: the wire calls a 2 KiB `.text` section and a 400 MiB JVM
    // heap the same unit, and a pair of rows is where fitting each to its own scale
    // costs nothing. See `size`.
    case "bytes":
      return size(raw);
  }
}

/** The winning cell is marked — with a weight and a background, never with colour alone. */
function cell(metric: Metric, side: "left" | "right"): string {
  return metric.verdict === side ? "numeric better" : "numeric";
}

/**
 * A column header of the metrics table. The architecture is named whenever the two sides
 * disagree about it: a column of milliseconds whose machine is not stated is the
 * thing that makes a cross-architecture table dangerous rather than merely useless.
 */
function Name({ identity, architecture }: { identity: Identity; architecture?: string }) {
  return (
    <span className="mode-tag">
      <span className="mode-dot" style={{ background: `var(${MODE_COLOR[identity.mode]})` }} />
      {label(identity)} · {identity.mode}
      {architecture !== undefined && <span className="side-architecture">{architecture}</span>}
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
  if (comparison.cross_isa) {
    return (
      <p className="compare-headline tie">
        On the run, the <strong>{winner.architecture}</strong> row came out {percent(run.gap_pct)}{" "}
        lower — and that is a fact about two machines, not about two backends. It is not a result,
        and this project does not publish it. See the warning above.
      </p>
    );
  }
  return (
    <p className="compare-headline">
      On the run, <strong>{label(winner)}</strong> in <strong>{winner.mode}</strong> is faster by{" "}
      <strong>{percent(run.gap_pct)}</strong>
      {run.noise_pct !== null && <>, against a dispersion of {percent(run.noise_pct)}</>}. A ratio
      within one campaign, on one architecture — the only cross-implementation claim this project
      publishes.
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

function Verdict({
  metric,
  left,
  right,
  crossIsa,
}: {
  metric: Metric;
  left: Identity;
  right: Identity;
  /** Across an architecture the two sides can be the same triple, and only the machine tells them apart. */
  crossIsa: { left: string; right: string } | null;
}) {
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
      if (crossIsa !== null) {
        const architecture = metric.verdict === "left" ? crossIsa.left : crossIsa.right;
        return (
          <span className="tie" title="two machines, not two backends">
            lower on <strong>{architecture}</strong>, by {percent(metric.gap_pct)} — not a result
          </span>
        );
      }
      return (
        <span>
          <strong>{label(winner)}</strong> · {winner.mode}, by {percent(metric.gap_pct)}
        </span>
      );
    }
  }
}

/**
 * The two rows were measured on two machines, and the page says so before the reader
 * reads a single number.
 *
 * The harness computed the comparison anyway — refusing would only send somebody off
 * to divide the two numbers by hand, with nothing on screen to tell them not to. So
 * the numbers are there, and so is this.
 */
function CrossIsaWarning({ left, right }: { left: string; right: string }) {
  const methodology = `${import.meta.env.BASE_URL}methodology/#flags-and-the-architecture-baseline`;
  return (
    <section className="warnings">
      <h2>
        These two rows ran on different machines — {left} and {right}
      </h2>
      <p>
        So the timings below are <strong>not a comparison</strong>, and the ratio between them is
        not a result. Two architectures means two CPUs, two clock speeds, two instruction sets and
        two memory systems: a millisecond here and a millisecond there answer different questions,
        and dividing one by the other describes neither. Whatever the verdict column says, it is
        ranking the machines at least as much as the backends.
      </p>
      <p>
        The ratio is the thing that travels. If you want to know whether Rust beats C <em>more</em>{" "}
        on {right} than on {left}, compare each of them against the same baseline <em>within</em>{" "}
        its own campaign, and compare the two ratios —{" "}
        <a href={methodology}>the architecture rule</a> is why, and it is short.
      </p>
      <p>
        One column does survive the crossing, and it is the reason this pairing is worth having:{" "}
        <strong>the checksum</strong>. In <code>strict</code> mode it is obliged to be bit-identical
        on every architecture, every compiler and every language. If the two below disagree, that is
        not a curiosity — it is a bug in one of them.
      </p>
    </section>
  );
}

interface PickerProps {
  side: "left" | "right";
  title: string;
  aggregates: Aggregate[];
  selected: Aggregate;
  /** The architecture of the campaign this side is reading. */
  architecture: string;
  architectures: string[];
  onPick: (side: "left" | "right", architecture: string, row: Aggregate) => void;
  onArch: (side: "left" | "right", architecture: string) => void;
}

/**
 * Language, then toolchain, then mode — three questions, in the order a reader asks
 * them.
 *
 * Each list is built from the rows this campaign actually measured, and each is
 * scoped by the answer above it: pick `python` and the toolchains are `cpython`,
 * `cython · cpython`, `pypy`, because those are the ones that ran. Changing a
 * language keeps the mode if the new language has it and falls back to its first row
 * if it does not — a selector that can land on a row the campaign never measured is
 * a selector that can produce a blank page.
 */
function Picker({
  side,
  title,
  aggregates,
  selected,
  architecture,
  architectures,
  onPick,
  onArch,
}: PickerProps) {
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
      onPick(side, architecture, row);
    }
  };

  const setToolchain = (chain: string) => {
    const row = nearest(sameLanguage.filter((candidate) => toolchain(candidate) === chain));
    if (row !== undefined) {
      onPick(side, architecture, row);
    }
  };

  const setMode = (mode: string) => {
    const row = sameLanguage.find(
      (candidate) => toolchain(candidate) === toolchain(selected) && candidate.mode === mode,
    );
    if (row !== undefined) {
      onPick(side, architecture, row);
    }
  };

  return (
    <div className="compare-side">
      <div className="compare-side-title">{title}</div>

      {architectures.length > 1 && (
        <label className="filter">
          <span>architecture — the machine it ran on</span>
          <select value={architecture} onChange={(event) => onArch(side, event.target.value)}>
            {architectures.map((candidate) => (
              <option key={candidate} value={candidate}>
                {candidate}
              </option>
            ))}
          </select>
        </label>
      )}

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
