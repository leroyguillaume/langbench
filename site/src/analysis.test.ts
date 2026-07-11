// The boundary, end to end: the campaign the site publishes, through the harness
// compiled to WebAssembly, into the schema the site validates against.
//
// This is the test that catches a drift the type checker cannot see. `analysis.rs`
// gains a field, or renames one, and every `tsc --noEmit` still passes — because
// TypeScript believes the zod schema, and the zod schema is the thing that is
// now wrong. Running the real WASM over the real file is the only way to find out.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { z } from "zod";
import { analysisSchema } from "./analysis";
import init, { analyze } from "./wasm/langbench.js";

// Resolved from the project root, not from `import.meta.url`: under jsdom the
// module URL is an `http://` one, and there is no file behind it.
const DATA = resolve(process.cwd(), "public/data");
const WASM = resolve(process.cwd(), "src/wasm/langbench_bg.wasm");

/** The campaigns this build publishes, exactly as the site discovers them. */
const published = (): string[] =>
  z.array(z.string()).parse(JSON.parse(readFileSync(resolve(DATA, "campaigns.json"), "utf8")));

let ndjson: string;

beforeAll(async () => {
  // The browser fetches the module over HTTP; here we hand it the bytes. Same
  // module, same code — `npm run build` produced both.
  await init({ module_or_path: readFileSync(WASM) });

  const [first] = published();
  if (first === undefined) {
    throw new Error("this build publishes no campaign");
  }
  ndjson = readFileSync(resolve(DATA, first), "utf8");
});

describe("the WebAssembly boundary", () => {
  it("summarizes the published campaign into the shape the site expects", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));

    expect(analysis.algos.length).toBeGreaterThan(0);
    expect(analysis.backends.length).toBeGreaterThan(0);

    const [algo] = analysis.algos;
    expect(algo?.aggregates.length).toBeGreaterThan(0);
  });

  it("ranks the aggregates fastest first, on the minimum wall-clock", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const measured = (analysis.algos[0]?.aggregates ?? [])
      .map((row) => row.run_wall?.min)
      .filter((min): min is number => min !== undefined && min !== null);

    expect(measured.length).toBeGreaterThan(1);
    expect([...measured].sort((a, b) => a - b)).toStrictEqual(measured);
  });

  /// The reason this whole boundary exists. A checksum is a 64-bit integer, and
  /// `JSON.parse` would have rounded it to the nearest double on the way in.
  it("hands the checksum over as a string, with every bit intact", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const checksum = analysis.algos[0]?.strict_checksum;

    expect(typeof checksum).toBe("string");
    expect(checksum).toMatch(/^\d+$/);

    // Whatever the campaign computed, every strict-mode row agreed on it: that is
    // the invariant the harness aborts a run over.
    for (const row of analysis.algos[0]?.aggregates ?? []) {
      if (row.mode === "strict") {
        expect(row.checksum).toBe(checksum);
        expect(row.checksum_delta).toBe("0");
      }
    }
  });

  it("aggregates the warmup rounds only when asked to, and never silently", () => {
    const cold = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const warm = analysisSchema.parse(analyze(ndjson, { include_warmup: true }));

    const samples = (analysis: typeof cold) => analysis.algos[0]?.aggregates[0]?.run_wall?.n ?? 0;

    expect(cold.options.include_warmup).toBe(false);
    expect(warm.options.include_warmup).toBe(true);
    expect(samples(warm)).toBeGreaterThan(samples(cold));
    // A warmup round can only ever be slower, so folding it in cannot lower the
    // minimum — it can only fail to raise it.
    const coldMin = cold.algos[0]?.aggregates[0]?.run_wall?.min ?? 0;
    const warmMin = warm.algos[0]?.aggregates[0]?.run_wall?.min ?? 0;
    expect(warmMin).toBeLessThanOrEqual(coldMin);
  });

  it("refuses a file that is not a campaign, rather than plotting nonsense", () => {
    expect(() => analyze("not ndjson at all", { include_warmup: false })).toThrow();
  });

  /// The ISA comes out of the machine record the campaign wrote, never out of the
  /// filename. An absolute timing never crosses an ISA, and the site keeps two
  /// campaigns apart on this field — so it had better be the machine's own word.
  it("reports the ISA of every published campaign, from inside the file", () => {
    for (const file of published()) {
      const raw = readFileSync(resolve(DATA, file), "utf8");
      const analysis = analysisSchema.parse(analyze(raw, { include_warmup: false }));

      expect(analysis.arch).not.toBe("");
      // The name is a convenience for a human reading `ls`. If it and the header
      // ever disagree, the header is right -- but they should not disagree, and a
      // campaign filed under the wrong ISA is worth catching here.
      expect(file).toBe(`${analysis.arch}.ndjson`);
    }
  });
});
