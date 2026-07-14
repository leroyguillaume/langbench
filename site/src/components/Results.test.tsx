// The campaign a route names is the campaign that renders — or the page says it cannot.
//
// The results island used to *choose* its campaign, out of a query string, with a chain
// of fallbacks widening to "whatever was published first". That is safe on a site with
// one campaign and a lie on a site with four: a page whose address says `x86_64` and
// whose numbers came from an AArch64 run is the single worst thing this project could
// publish, and no chart on it would say so.
//
// Now the route names it and the *header* answers. This test is what holds that line:
// the real WASM, over a real committed campaign, through the real island.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { fetchCampaign, type LoadedCampaign } from "../analysis";
import init from "../wasm/langbench.js";
import { Results } from "./Results";

const WASM = resolve(process.cwd(), "src/wasm/langbench_bg.wasm");
const FIXTURE = resolve(process.cwd(), "src/__fixtures__/aarch64.ndjson");

let loaded: LoadedCampaign;

// The island reads its campaigns through this hook, which fetches them. The fetch is
// the boundary being stubbed — the analysis under it is the harness's own, over the
// committed samples, exactly as in a browser.
const campaigns = vi.hoisted(() => ({ value: [] as LoadedCampaign[] }));
vi.mock("../campaigns", () => ({
  useCampaigns: () => ({ campaigns: campaigns.value, error: null, pending: false }),
}));

beforeAll(async () => {
  await init({ module_or_path: readFileSync(WASM) });
  const ndjson = readFileSync(FIXTURE, "utf8");
  vi.stubGlobal("fetch", async () => new Response(ndjson));
  loaded = await fetchCampaign("/data/mandelbrot/aarch64.ndjson", { include_warmup: false });
  campaigns.value = [loaded];
});

describe("the campaign a route names", () => {
  it("renders the campaign whose header matches the route", () => {
    render(<Results workload="mandelbrot" architecture="aarch64" />);

    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("mandelbrot on aarch64");
    // And the numbers under it are this campaign's: every row the harness aggregated
    // out of the fixture is in the table, each named by the triple that produced it.
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBeGreaterThan(loaded.analysis.backends.length);
    expect(screen.getAllByText(/rust/).length).toBeGreaterThan(0);
  });

  // The one failure mode worth a test of its own. A page that fell back to another
  // campaign would publish real numbers under the wrong machine's name — which is
  // exactly what a reader cannot detect, because every number on it would be
  // internally consistent.
  it("refuses to fall back to another campaign, and says why", () => {
    render(<Results workload="mandelbrot" architecture="x86_64" />);

    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "This campaign is not in the build",
    );
    expect(screen.queryByText(/on aarch64/)).toBeNull();
  });

  it("refuses a workload this campaign never measured", () => {
    render(<Results workload="nbody" architecture="aarch64" />);

    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "This campaign is not in the build",
    );
  });
});
