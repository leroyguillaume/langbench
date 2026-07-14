// Moving a side to the other machine has to actually move it.
//
// It did not, and the way it failed is the interesting part. Every control on this page
// names a campaign by `(workload, architecture)`, and the workload came out of the query
// string — where it is `null` for anybody who arrived from the sidebar rather than from a
// campaign's "Compare" link. A lookup for the workload `null` matches nothing, so the
// architecture picker searched for a campaign that cannot exist, found none, and returned.
// The page rendered, the dropdown moved, and nothing happened: no error, no empty state,
// no clue.
//
// So the workload is resolved from the campaign that actually loaded, and this is the
// test that says so. Two campaigns, because one campaign cannot express the bug.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { fetchCampaign, type LoadedCampaign } from "../analysis";
import init from "../wasm/langbench.js";
import { ComparePage } from "./ComparePage";

const WASM = resolve(process.cwd(), "src/wasm/langbench_bg.wasm");
const FIXTURE = resolve(process.cwd(), "src/__fixtures__/aarch64.ndjson");

const campaigns = vi.hoisted(() => ({ value: [] as LoadedCampaign[] }));
vi.mock("../campaigns", () => ({
  useCampaigns: () => ({ campaigns: campaigns.value, error: null, pending: false }),
}));

/**
 * The same campaign, on another machine.
 *
 * Only the *header's* architecture is rewritten, because that is the only place the site
 * reads one from — the samples under it are untouched, and so are their timings. It is a
 * fixture for a routing question, not a measurement: putting these two side by side would
 * be exactly the cross-architecture comparison the harness flags, which is what the second
 * assertion below is watching for.
 */
function onAnotherMachine(ndjson: string): string {
  const [header, ...samples] = ndjson.split("\n");
  return [
    (header ?? "").replace('"architecture":"aarch64"', '"architecture":"x86_64"'),
    ...samples,
  ].join("\n");
}

async function load(ndjson: string): Promise<LoadedCampaign> {
  vi.stubGlobal("fetch", async () => new Response(ndjson));
  return fetchCampaign("/data/mandelbrot/whatever.ndjson", { include_warmup: false });
}

beforeAll(async () => {
  await init({ module_or_path: readFileSync(WASM) });
  const ndjson = readFileSync(FIXTURE, "utf8");
  campaigns.value = [await load(ndjson), await load(onAnotherMachine(ndjson))];
});

describe("the compare page", () => {
  // No query string: the reader clicked "Compare" in the sidebar, and named no campaign.
  beforeAll(() => window.history.replaceState(null, "", "/compare/"));

  it("moves a side to the other architecture, with no campaign named in the URL", () => {
    render(<ComparePage />);

    const [left] = screen.getAllByRole("combobox", { name: /architecture/ });
    expect(left).toBeDefined();
    expect((left as HTMLSelectElement).value).toBe("aarch64");

    fireEvent.change(left as HTMLSelectElement, { target: { value: "x86_64" } });

    expect(
      (screen.getAllByRole("combobox", { name: /architecture/ })[0] as HTMLSelectElement).value,
    ).toBe("x86_64");
    // And the harness is told the pair now crosses an architecture, which is the whole
    // reason a side is allowed to name its own machine.
    expect(screen.getByText(/two architectures means two CPUs/i)).toBeDefined();
  });
});
