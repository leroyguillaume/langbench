// What the site knows before a browser has fetched anything: the work, and which
// campaigns exist.
//
// Written by `scripts/data.js` into `src/generated/site.json` and imported here at
// build time — never fetched. It is what the routes are made of (`/workloads/mandelbrot/`,
// `/workloads/mandelbrot/x86_64/`) and what the sidebar lists, and both have to exist
// as files on a server that only serves files.
//
// It carries **no measurement**. The workloads are the manifests as the harness reads
// them, and a campaign is named here by its workload, its architecture and its host —
// four strings out of a header. Every number on this site still comes out of the WASM,
// out of the samples, in the browser.
//
// Validated at the boundary like everything else that crosses into TypeScript: a
// generated file is still a file, and a field that quietly changed shape upstream
// should fail the build rather than render an empty page.

import { z } from "zod";
import generated from "./generated/site.json";

/** One knob of a workload, and the value the manifest declares. */
const paramSchema = z.object({
  name: z.string(),
  value: z.union([z.number(), z.string(), z.boolean()]),
});

/**
 * A backend that does the work, as its `bench.yaml` declares it.
 *
 * The triple is the identity — no name, no slug — and each half of the toolchain may be
 * absent: gcc compiles and nothing interprets, CPython interprets and nothing compiles
 * ahead of the run, Cython does both. An absence is a published fact, so it is rendered
 * rather than blanked.
 *
 * These are the *declared* backends, which is not the same set as the rows of any
 * campaign: one declared today has no numbers yet, and one that crashed has a failure
 * instead of a row. That is the point of describing them here — on the page about the
 * work — rather than under a table of results that may not contain them.
 */
export const backendSchema = z.object({
  language: z.string(),
  compiler: z.string().nullable(),
  interpreter: z.string().nullable(),
  description: z.string(),
  comments: z.string().nullable(),
  /**
   * The machines this backend can be built on at all.
   *
   * Not a preference: a toolchain either exists for an architecture or it does not.
   * Kotlin/Native publishes no `linux-aarch64` host compiler, and the two ways around
   * that — emulation, cross-building — are both forbidden here, so a campaign on the
   * other machine skips the row. Published on the card so that a reader who finds a
   * backend missing from one campaign and present in another learns *why* on the page
   * about the work, instead of concluding that the campaign lost it.
   */
  architectures: z.array(z.string()),
});

/**
 * A workload as its `workload.yaml` declares it *today*, and the backends that implement it.
 *
 * Not what a campaign measured: that is the snapshot inside the campaign's own header,
 * and the two are allowed to differ. This is the work; that is the run.
 *
 * `checksum` is a string because it is a 64-bit integer and a JavaScript number is a
 * double. It is displayed and compared, never added.
 */
export const workloadSchema = z.object({
  id: z.string(),
  description: z.string(),
  /** Directory names. The harness's business: never displayed, never linked. */
  implementations: z.array(z.string()),
  backends: z.array(backendSchema),
  params: z.array(paramSchema),
  checksum: z.string().nullable(),
});

/** A campaign this build publishes, named by what its header said. */
export const campaignSchema = z.object({
  /** Its path under `public/data/`, which is also how an island fetches it. */
  file: z.string(),
  workload: z.string(),
  architecture: z.string(),
  timestamp: z.string().nullable(),
  hostname: z.string().nullable(),
});

const siteSchema = z.object({
  workloads: z.array(workloadSchema),
  campaigns: z.array(campaignSchema),
});

export type Workload = z.infer<typeof workloadSchema>;
export type Backend = z.infer<typeof backendSchema>;
export type CampaignRef = z.infer<typeof campaignSchema>;

const site = siteSchema.parse(generated);

/** Every workload declared on disk, alphabetically — including one nobody has measured yet. */
export const workloads: Workload[] = [...site.workloads].sort((left, right) =>
  left.id.localeCompare(right.id),
);

/** Every campaign this build publishes, by workload and then by architecture. */
export const campaigns: CampaignRef[] = [...site.campaigns].sort(
  (left, right) =>
    left.workload.localeCompare(right.workload) ||
    left.architecture.localeCompare(right.architecture),
);

/** The campaigns measured on one workload. Empty is a legitimate answer, and a page says so. */
export function campaignsOf(workload: string): CampaignRef[] {
  return campaigns.filter((campaign) => campaign.workload === workload);
}

/**
 * A description as its author wrote it: paragraphs, separated by a blank line.
 *
 * The manifest is YAML and its `description` is one string, so the blank lines are the
 * only structure it has. Rendering it into one `<p>` would hand the reader a wall.
 */
export function paragraphs(text: string): string[] {
  return text
    .split(/\n\s*\n/)
    .map((paragraph) => paragraph.trim())
    .filter((paragraph) => paragraph !== "");
}

/**
 * A paragraph may open with a run-in heading: a short label sentence the author uses to
 * structure a long description ("What it puts under the light."). It is set in bold and
 * the body follows on the same line, the way the cards already lead with a bold phrase.
 *
 * The signal is deliberately narrow -- a brief opening sentence with no internal
 * punctuation, ending in a single period -- so ordinary prose that happens to start with
 * a short sentence is left alone. This is formatting, never structure: the manifest is
 * still one YAML string whose only real structure is its blank lines.
 */
export function runIn(paragraph: string): { lead?: string; body: string } {
  const match = paragraph.match(/^([^.?!,;:]{1,40}\.)\s+(.+)$/s);
  if (!match) {
    return { body: paragraph };
  }
  const [, lead, body] = match;
  return lead && body ? { lead, body } : { body: paragraph };
}
