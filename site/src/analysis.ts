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
import init, { analyze as analyzeWasm } from "./wasm/langbench.js";

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
  algo: z.string(),
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
  build_elapsed: summarySchema.nullable(),
  binary_bytes: z.number().nullable(),
  binary_stripped_bytes: z.number().nullable(),
  text_bytes: z.number().nullable(),
  checksum: checksumSchema,
  checksum_delta: checksumSchema,
});

export const backendSchema = z.object({
  id: z.string(),
  algo: z.string(),
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
  algo: z.string(),
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

export const campaignSchema = z.object({
  langbench_version: z.string(),
  timestamp: z.string(),
  cpu: z.number().int(),
  grid_size: z.number().int(),
  max_iter: z.number().int(),
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
   * The ISA the campaign ran on, out of the machine record inside the file — never
   * out of the name of the file. An absolute timing never crosses an ISA, and the
   * check that enforces it cannot rest on what somebody called a file.
   */
  arch: z.string(),
  hostname: z.string().nullable(),
  machine_fields: z.array(z.object({ label: z.string(), value: z.string() })),
  /** Every reason the host was a poor benchmark target. It travels with the numbers. */
  warnings: z.array(z.string()),
  algos: z.array(
    z.object({
      algo: z.string(),
      strict_checksum: checksumSchema,
      aggregates: z.array(aggregateSchema),
    }),
  ),
  backends: z.array(backendSchema),
  /** Every backend the campaign lost. Empty on a campaign where everything worked. */
  failures: z.array(failureSchema),
});

export type Summary = z.infer<typeof summarySchema>;
export type FpMode = z.infer<typeof fpModeSchema>;
export type Aggregate = z.infer<typeof aggregateSchema>;
export type Backend = z.infer<typeof backendSchema>;
export type Failure = z.infer<typeof failureSchema>;
export type Campaign = z.infer<typeof campaignSchema>;
export type Analysis = z.infer<typeof analysisSchema>;
export type AlgoAnalysis = Analysis["algos"][number];

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
 * Fetch a campaign and summarize it — with the harness's own arithmetic.
 *
 * `campaignUrl` is served as text and never parsed by JavaScript. See the header.
 */
export async function fetchAnalysis(campaignUrl: string, options: Options): Promise<Analysis> {
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
    arch: analysis.arch,
    algos: analysis.algos.length,
    backends: analysis.backends.length,
    warnings: analysis.warnings.length,
    include_warmup: options.include_warmup,
  });
  return analysis;
}

/**
 * Every campaign this build publishes, summarized — one per ISA.
 *
 * They are kept apart on purpose. **An absolute timing never crosses an ISA**
 * (`METHODOLOGY.md#the-isa-rule`): an x86-64 millisecond and an aarch64
 * millisecond are not the same claim, and a chart that puts them in one bar
 * group invites exactly the comparison the methodology forbids. So the site
 * loads them all and shows one at a time.
 */
export async function fetchCampaigns(baseUrl: string, options: Options): Promise<Analysis[]> {
  const response = await fetch(`${baseUrl}campaigns.json`);
  if (!response.ok) {
    throw new Error(`fetching the campaign index: ${response.status} ${response.statusText}`);
  }
  const files = campaignsSchema.parse(await response.json());

  const analyses = await Promise.all(
    files.map((file) => fetchAnalysis(`${baseUrl}${file}`, options)),
  );
  // Deterministic order, and it is not the order the files were listed in: the
  // ISA is what the reader picks between, so the ISA is what sorts them.
  return analyses.sort((left, right) => left.arch.localeCompare(right.arch));
}
