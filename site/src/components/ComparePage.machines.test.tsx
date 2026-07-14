// Two rows from two machines: the machines are the confounding variable, so they are shown.
//
// The page already refused to call the crossing a result. What it did not do was say what
// *changed* — and "these ran on different machines" is a rule to obey, where two columns of
// kernel versions and clock speeds is a reason to believe it.

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
 * The same campaign, on another machine — a different architecture, on a differently named
 * host running a different kernel.
 *
 * Only the *header* is rewritten: the samples under it are untouched, and so are their
 * timings. That is exactly the shape of the mistake this page exists to refuse — two sets of
 * numbers that look comparable and are not — so it is the right fixture for asking whether
 * the page says so.
 */
function onAnotherMachine(ndjson: string): string {
  const [header, ...samples] = ndjson.split("\n");
  const moved = (header ?? "")
    .replace('"architecture":"aarch64"', '"architecture":"x86_64"')
    // The fixture is a campaign from a laptop: most of the machine record is `null`,
    // which is itself a fact the harness recorded. Two of the fields are given values so
    // that the two columns genuinely disagree — which is what this test is about.
    .replace(/"kernel":(null|"[^"]*")/, '"kernel":"6.8.0-45-generic"')
    .replace(/"hostname":(null|"[^"]*")/, '"hostname":"other-host"');
  return [moved, ...samples].join("\n");
}

async function load(ndjson: string): Promise<LoadedCampaign> {
  vi.stubGlobal("fetch", async () => new Response(ndjson));
  return fetchCampaign("/data/mandelbrot/whatever.ndjson", { include_warmup: false });
}

beforeAll(async () => {
  await init({ module_or_path: readFileSync(WASM) });
  const ndjson = readFileSync(FIXTURE, "utf8");
  campaigns.value = [await load(ndjson), await load(onAnotherMachine(ndjson))];
  window.history.replaceState(null, "", "/compare/");
});

describe("a pair that crosses an architecture", () => {
  it("shows the two machines, and marks what differs between them", () => {
    render(<ComparePage />);

    // Within one campaign there is one machine, and it is on the campaign's own page.
    expect(screen.queryByText("The two machines")).toBeNull();

    const [left] = screen.getAllByRole("combobox", { name: /architecture/ });
    fireEvent.change(left as HTMLSelectElement, { target: { value: "x86_64" } });

    expect(screen.getByText("The two machines")).toBeInTheDocument();
    // The kernel is a row where the two hosts disagree — and a disagreement is marked,
    // never left for the eye to find among twenty near-identical rows.
    const kernel = screen.getByText("6.8.0-45-generic");
    expect(kernel).toHaveClass("differs");
    expect(kernel.closest("tr")?.textContent).toContain("Kernel");
  });
});
