// The boundary between the harness and the browser.
//
// Two rules govern this file, and both are load-bearing:
//
// The field names are `snake_case` throughout, because they are the harness's
// own — `samples.ndjson` spells them that way, and so does the Rust struct this
// object is deserialized from. One vocabulary, no translation table.
//
// 1. **The site never computes a statistic.** Min-of-N, the buckets, the
//    definition of startup — all of it comes out of `src/analysis.rs`, compiled
//    to WebAssembly. A TypeScript re-implementation would be a second definition
//    of what `langbench` measures, and the two would drift the first time one of
//    them was "fixed". What is left here is formatting, sorting and drawing.
//
// 2. **The site never calls `JSON.parse` on a campaign.** `checksum` is a 64-bit
//    integer and a JavaScript number is a double: `JSON.parse` rounds every
//    integer past 2^53 without a word. So `samples.ndjson` is fetched as *text*
//    and handed straight to the WASM, which parses it with `serde_json` and hands
//    back checksums as strings. The site displays them and compares them; it
//    never does arithmetic on them.

import { z } from "zod";
import { logger } from "./logger";
import init, {
  analyze as analyzeWasm,
  compare_across as compareAcrossWasm,
  compare as compareWasm,
} from "./wasm/langbench.js";

/** Min-of-N and its dispersion, as `src/stats.rs` defines them. */
export const summarySchema = z.object({
  n: z.number().int(),
  min: z.number(),
  median: z.number(),
  /** Median absolute deviation, in the samples' own unit. */
  mad: z.number(),
  /** MAD as a percentage of the median. Past ~2%, percentage-level claims die. */
  mad_pct: z.number(),
});

export const fpModeSchema = z.enum(["strict", "fma", "fast"]);

/**
 * A checksum, as it crosses the wire: a string, always. See the header.
 */
const checksumSchema = z.string().nullable();

export const aggregateSchema = z.object({
  workload: z.string(),
  backend: z.string(),
  backend_id: z.string(),
  language: z.string(),
  /** `null` for a backend that compiles nothing ahead of the run — a published fact. */
  compiler: z.string().nullable(),
  /** `null` for a backend that ships machine code and no runtime. */
  interpreter: z.string().nullable(),
  mode: fpModeSchema,
  run_wall: summarySchema.nullable(),
  run_elapsed: summarySchema.nullable(),
  run_startup: summarySchema.nullable(),
  run_cpu_usec: summarySchema.nullable(),
  /**
   * Cores kept busy, in thousandths of a core — read against `cpu`, the thread count
   * the harness handed this kernel.
   *
   * The **median**, and it is the one number on the row that is not a minimum.
   * Contention inflates a spinning thread's CPU clock and the compute clock alike, in
   * both directions, so there is no one-sided noise to argue from and no reason the
   * extreme should be the estimate.
   * See `METHODOLOGY.md#parallel-efficiency-is-a-median-not-a-minimum`.
   */
  run_cores: summarySchema.nullable(),
  /** The container's peak memory, min-of-N. */
  run_peak_bytes: summarySchema.nullable(),
  build_elapsed: summarySchema.nullable(),
  build_cores: summarySchema.nullable(),
  build_peak_bytes: summarySchema.nullable(),
  /** The thread count this campaign handed every kernel. The denominator of `run_cores`. */
  cpu: z.number().int(),
  /** Bytes of the kernel's one source file — a property of the *language*, not of the backend. */
  source_bytes: z.number().nullable(),
  binary_bytes: z.number().nullable(),
  binary_stripped_bytes: z.number().nullable(),
  text_bytes: z.number().nullable(),
  checksum: checksumSchema,
  checksum_delta: checksumSchema,
});

export const backendSchema = z.object({
  id: z.string(),
  workload: z.string(),
  backend: z.string(),
  language: z.string(),
  compiler: z.string().nullable(),
  interpreter: z.string().nullable(),
  description: z.string(),
  comments: z.string().nullable(),
});

/**
 * A backend the campaign lost, and what it lost it to.
 *
 * Not a sample: it carries no timing and nothing aggregates it. It is on the wire
 * for the same reason `warnings` is — a table cannot be read without knowing what
 * is *not* in it. A backend that crashed has no row, and a row that is absent looks
 * exactly like a backend nobody ever wrote.
 */
export const failureSchema = z.object({
  workload: z.string(),
  backend: z.string(),
  backend_id: z.string(),
  language: z.string(),
  compiler: z.string().nullable(),
  interpreter: z.string().nullable(),
  description: z.string(),
  comments: z.string().nullable(),
  mode: fpModeSchema,
  /** `prepare`: the image never built. `measure`: it built, and the run went wrong. */
  stage: z.enum(["prepare", "measure"]),
  phase: z.enum(["build", "run"]).nullable(),
  /** Zero-based, as the harness counts. Absent when the image never built. */
  round: z.number().int().nullable(),
  error: z.string(),
});

/** One knob of a workload, and the value the campaign ran it at. */
export const paramSchema = z.object({
  name: z.string(),
  value: z.union([z.number(), z.string(), z.boolean()]),
});

/**
 * The workload a campaign measured, snapshotted into its header when it started.
 *
 * The site reads campaigns and nothing else — it never sees a `workload.yaml`. This
 * is how it knows what the work was, how it was sized, and what the right answer
 * is. And because it is a snapshot, editing the manifest afterwards cannot rewrite
 * what a campaign from three months ago says it measured.
 *
 * `strict_checksum` is a string here for the reason every checksum on this wire is:
 * it is a 64-bit integer, and a JavaScript number is a double.
 */
export const workloadSchema = z.object({
  id: z.string(),
  description: z.string(),
  implementations: z.array(z.string()),
  params: z.array(paramSchema),
  strict_checksum: z.string().nullable(),
});

export const campaignSchema = z.object({
  langbench_version: z.string(),
  timestamp: z.string(),
  workload: workloadSchema,
  cpu: z.number().int(),
  rounds: z.number().int(),
  build_rounds: z.number().int(),
  warmup_rounds: z.number().int(),
  march: z.string(),
  modes: z.array(z.string()),
});

export const analysisSchema = z.object({
  campaign: campaignSchema,
  options: z.object({ include_warmup: z.boolean() }),
  /**
   * The architecture the campaign ran on, out of the machine record inside the file — never
   * out of the name of the file. An absolute timing never crosses an architecture, and the
   * check that enforces it cannot rest on what somebody called a file.
   */
  architecture: z.string(),
  hostname: z.string().nullable(),
  machine_fields: z.array(z.object({ label: z.string(), value: z.string() })),
  /** Every reason the host was a poor benchmark target. It travels with the numbers. */
  warnings: z.array(z.string()),
  workloads: z.array(
    z.object({
      workload: z.string(),
      strict_checksum: checksumSchema,
      aggregates: z.array(aggregateSchema),
    }),
  ),
  backends: z.array(backendSchema),
  /** Every backend the campaign lost. Empty on a campaign where everything worked. */
  failures: z.array(failureSchema),
});

/**
 * One row of a head-to-head, as `src/compare.rs` decides it.
 *
 * `ratio`, `gap_pct` and `noise_pct` are the harness's arithmetic, not the site's:
 * whether a gap is large enough to be a difference is a definition of what this
 * project measures, and it has one home. The site spells these numbers out. It
 * does not compute them, and it does not second-guess the `verdict`.
 */
export const metricSchema = z.object({
  key: z.string(),
  label: z.string(),
  /**
   * What the two values are measured in. The site spells it; it never converts it.
   *
   * A **closed set**, deliberately: an unknown unit fails the parse rather than
   * degrading to an unformatted number. A new unit lands in the same change as the
   * renderer that can spell it — on the wire ahead of it, it would take the whole
   * head-to-head down on a page that is live today.
   */
  unit: z.enum(["nanoseconds", "microseconds", "bytes"]),
  left: z.number().nullable(),
  right: z.number().nullable(),
  /** `right / left`. Below 1, the right-hand backend is the smaller one. */
  ratio: z.number().nullable(),
  /** The gap, as a percentage of the smaller of the two. Always positive. */
  gap_pct: z.number().nullable(),
  /** The dispersion the pair carries — the bar `gap_pct` has to clear to be a result. */
  noise_pct: z.number().nullable(),
  /** `tie`: the gap is inside the noise. Not "equal" — *indistinguishable*. */
  verdict: z.enum(["left", "right", "tie", "unmeasured"]),
});

export const sideSchema = z.object({
  backend: z.string(),
  backend_id: z.string(),
  language: z.string(),
  compiler: z.string().nullable(),
  interpreter: z.string().nullable(),
  mode: fpModeSchema,
  /** The architecture of the campaign this row was measured on. */
  architecture: z.string(),
});

export const comparisonSchema = z.object({
  workload: z.string(),
  left: sideSchema,
  right: sideSchema,
  metrics: z.array(metricSchema),
  checksums: z.object({
    left: checksumSchema,
    right: checksumSchema,
    /** `null` when either side never reported one. Compared in Rust, on the full 64 bits. */
    same: z.boolean().nullable(),
    /** Two `strict` rows that disagree. The harness aborts over it; a file carrying one did not come from here. */
    violates_strict_invariant: z.boolean(),
  }),
  /**
   * The two rows come from two architectures, and **every timing above is therefore
   * meaningless as a comparison**. The harness decides this, and the site's only job
   * is to say it out loud: a ratio travels between architectures, a millisecond does not.
   * See `METHODOLOGY.md#the-architecture-rule`.
   *
   * The checksums are the exception, and the reason the crossing is worth offering:
   * in `strict` mode they are obliged to be bit-identical on x86-64 and on AArch64
   * alike, and a divergence is a bug in one of them.
   */
  cross_isa: z.boolean(),
});

export type Summary = z.infer<typeof summarySchema>;
export type FpMode = z.infer<typeof fpModeSchema>;
export type Aggregate = z.infer<typeof aggregateSchema>;
export type Backend = z.infer<typeof backendSchema>;
export type Failure = z.infer<typeof failureSchema>;
export type Campaign = z.infer<typeof campaignSchema>;
export type Analysis = z.infer<typeof analysisSchema>;
export type WorkloadAnalysis = Analysis["workloads"][number];
export type Metric = z.infer<typeof metricSchema>;
export type Side = z.infer<typeof sideSchema>;
export type Comparison = z.infer<typeof comparisonSchema>;

/** One side of a pair, named the way the samples name it — never by row index. */
export interface Row {
  backend: string;
  mode: FpMode;
}

/** The pair a reader asked for. `snake_case`: it is deserialized straight into Rust. */
export interface Selection {
  workload: string;
  left: Row;
  right: Row;
}

/**
 * Every knob the site has. `snake_case` because it crosses into Rust: this object
 * is deserialized straight into `analysis::Options`.
 */
export interface Options {
  /** Warmup rounds are always recorded. This decides whether they are aggregated. */
  include_warmup: boolean;
}

/** The index `scripts/data.js` writes: the campaigns this build publishes. */
const campaignsSchema = z.array(z.string());

let ready: Promise<void> | undefined;

/** Instantiate the WebAssembly module once, however many callers ask for it. */
function load(): Promise<void> {
  if (ready === undefined) {
    ready = init().then(() => {
      logger.debug("wasm.ready");
    });
  }
  return ready;
}

/**
 * A campaign, as the site holds it: the file, and the harness's summary of it.
 *
 * The raw NDJSON is kept beside the analysis rather than dropped, because it is
 * the *input* of every other question the harness can be asked about this
 * campaign — `compare()` below is the second one. The file is the only artefact
 * a run produces and the only thing that cannot be recomputed; everything else on
 * this page is derived from it, in Rust, on demand.
 */
export interface LoadedCampaign {
  /** The campaign, byte for byte, as `samples/<architecture>.ndjson` on disk. */
  ndjson: string;
  analysis: Analysis;
}

/**
 * Fetch a campaign and summarize it — with the harness's own arithmetic.
 *
 * `campaignUrl` is served as text and never parsed by JavaScript. See the header.
 */
export async function fetchCampaign(
  campaignUrl: string,
  options: Options,
): Promise<LoadedCampaign> {
  await load();

  const response = await fetch(campaignUrl);
  if (!response.ok) {
    throw new Error(`fetching ${campaignUrl}: ${response.status} ${response.statusText}`);
  }
  // `.text()`, deliberately. `.json()` would round every checksum past 2^53.
  const ndjson = await response.text();
  logger.debug("campaign.fetched", { url: campaignUrl, bytes: ndjson.length });

  const raw: unknown = analyzeWasm(ndjson, options);
  const analysis = analysisSchema.parse(raw);
  logger.debug("campaign.analyzed", {
    url: campaignUrl,
    architecture: analysis.architecture,
    workloads: analysis.workloads.length,
    backends: analysis.backends.length,
    warnings: analysis.warnings.length,
    include_warmup: options.include_warmup,
  });
  return { ndjson, analysis };
}

/**
 * Every campaign this build publishes, summarized — one per architecture.
 *
 * They are kept apart on purpose. **An absolute timing never crosses an architecture**
 * (`METHODOLOGY.md#the-architecture-rule`): an x86-64 millisecond and an aarch64
 * millisecond are not the same claim, and a chart that puts them in one bar
 * group invites exactly the comparison the methodology forbids. So the site
 * loads them all and shows one at a time.
 */
export async function fetchCampaigns(baseUrl: string, options: Options): Promise<LoadedCampaign[]> {
  const response = await fetch(`${baseUrl}campaigns.json`);
  if (!response.ok) {
    throw new Error(`fetching the campaign index: ${response.status} ${response.statusText}`);
  }
  const files = campaignsSchema.parse(await response.json());

  const campaigns = await Promise.all(
    files.map((file) => fetchCampaign(`${baseUrl}${file}`, options)),
  );
  // Deterministic order, and it is not the order the files were listed in: the
  // architecture is what the reader picks between, so the architecture is what sorts them.
  return campaigns.sort((left, right) =>
    left.analysis.architecture.localeCompare(right.analysis.architecture),
  );
}

/**
 * Two rows of one campaign, head to head — and whether the gap is a difference.
 *
 * Every number it returns is the harness's: the ratio, the gap, the dispersion
 * the gap has to clear, and the verdict when it does not. A gap smaller than the
 * noise the campaign carries is a **tie**, however different the two minima look
 * — and deciding that is `src/compare.rs`'s job, not this file's. See
 * `METHODOLOGY.md#a-difference-smaller-than-the-dispersion-is-not-a-difference`.
 *
 * Synchronous, and the WASM has to be up: every caller reaches this through a
 * campaign that `fetchCampaign` already loaded. Throws on a row the campaign
 * never measured, rather than comparing against an invented zero.
 */
export function compare(ndjson: string, options: Options, selection: Selection): Comparison {
  const raw: unknown = compareWasm(ndjson, options, selection);
  return comparisonSchema.parse(raw);
}

/**
 * The same, with each row drawn from a campaign of its own — which is how a reader
 * puts x86-64 next to AArch64.
 *
 * The comparison comes back with `cross_isa` set, and the page is obliged to say so.
 * The harness computes the timings exactly as it does within one campaign, because
 * refusing would only send somebody off to divide the two numbers by hand, with
 * nothing on screen to tell them not to — and they mean nothing across two machines.
 */
export function compareAcross(
  leftNdjson: string,
  rightNdjson: string,
  options: Options,
  selection: Selection,
): Comparison {
  const raw: unknown = compareAcrossWasm(leftNdjson, rightNdjson, options, selection);
  return comparisonSchema.parse(raw);
}
