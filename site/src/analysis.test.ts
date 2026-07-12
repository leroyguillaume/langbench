// The boundary, end to end: the campaign the site publishes, through the harness
// compiled to WebAssembly, into the schema the site validates against.
//
// This is the test that catches a drift the type checker cannot see. `analysis.rs`
// gains a field, or renames one, and every `tsc --noEmit` still passes — because
// TypeScript believes the zod schema, and the zod schema is the thing that is
// now wrong. Running the real WASM over the real file is the only way to find out.

import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { analysisSchema, comparisonSchema } from "./analysis";
import init, { analyze, compare } from "./wasm/langbench.js";

// Resolved from the project root, not from `import.meta.url`: under jsdom the
// module URL is an `http://` one, and there is no file behind it.
const WASM = resolve(process.cwd(), "src/wasm/langbench_bg.wasm");

// Real campaigns, committed: `langbench run` output, headers and failures and all.
//
// Not `public/data`. What the repository *publishes* is a question of which
// machine last produced numbers worth publishing, and the answer is legitimately
// "none yet" — `samples/` can be empty, and `samples.local/` is never committed.
// This test asks a different question, and it has to be able to ask it on a
// checkout that publishes nothing.
//
// Raw samples, byte for byte, because that is the only input this boundary has
// ever been given. A hand-written or trimmed fixture would agree with the schema
// by construction, and agreeing with the schema is the one thing this test must
// not assume.
const FIXTURES = resolve(process.cwd(), "src/__fixtures__");
const campaigns = (): string[] => readdirSync(FIXTURES).filter((f) => f.endsWith(".ndjson"));

let ndjson: string;

beforeAll(async () => {
  // The browser fetches the module over HTTP; here we hand it the bytes. Same
  // module, same code — `npm run wasm` produced both.
  await init({ module_or_path: readFileSync(WASM) });

  const [first] = campaigns();
  if (first === undefined) {
    throw new Error(`no campaign fixture in ${FIXTURES}`);
  }
  ndjson = readFileSync(resolve(FIXTURES, first), "utf8");
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

  /// The head-to-head is the harness's arithmetic too. The site picks two rows and
  /// spells the answer; it does not decide whether a gap is a difference.
  it("compares two rows of the published campaign, and hands back the harness's verdict", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const algo = analysis.algos[0];
    const [first, second] = algo?.aggregates ?? [];
    if (algo === undefined || first === undefined || second === undefined) {
      throw new Error("the fixture campaign measured fewer than two rows");
    }

    const selection = {
      algo: algo.algo,
      left: { backend: first.backend, mode: first.mode },
      right: { backend: second.backend, mode: second.mode },
    };
    const comparison = comparisonSchema.parse(
      compare(ndjson, { include_warmup: false }, selection),
    );

    expect(comparison.left.backend).toBe(first.backend);
    expect(comparison.right.backend).toBe(second.backend);

    const run = comparison.metrics.find((metric) => metric.key === "run");
    // The aggregates arrive fastest first, so the runner-up is never the smaller
    // number: the ratio of the two is at least one.
    expect(run?.ratio ?? 0).toBeGreaterThanOrEqual(1);
    expect(run?.left).toBe(first.run_wall?.min);
    expect(run?.right).toBe(second.run_wall?.min);
    expect(["left", "tie"]).toContain(run?.verdict);
  });

  /// A comparison is a property of the pair, not of the order the reader picked
  /// them in.
  it("inverts the ratio when the two rows are swapped, and nothing else", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const algo = analysis.algos[0];
    const [first, second] = algo?.aggregates ?? [];
    if (algo === undefined || first === undefined || second === undefined) {
      throw new Error("the fixture campaign measured fewer than two rows");
    }

    const left = { backend: first.backend, mode: first.mode };
    const right = { backend: second.backend, mode: second.mode };
    const forward = comparisonSchema.parse(
      compare(ndjson, { include_warmup: false }, { algo: algo.algo, left, right }),
    );
    const backward = comparisonSchema.parse(
      compare(ndjson, { include_warmup: false }, { algo: algo.algo, left: right, right: left }),
    );

    const run = (comparison: typeof forward) =>
      comparison.metrics.find((metric) => metric.key === "run");
    expect((run(forward)?.ratio ?? 0) * (run(backward)?.ratio ?? 0)).toBeCloseTo(1, 6);
    expect(run(forward)?.gap_pct).toBe(run(backward)?.gap_pct);
  });

  it("refuses a row the campaign never measured, rather than comparing against a zero", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const algo = analysis.algos[0]?.algo ?? "mandelbrot";
    const first = analysis.algos[0]?.aggregates[0];

    expect(() =>
      compare(
        ndjson,
        { include_warmup: false },
        {
          algo,
          left: {
            backend: first?.backend ?? "c-gcc",
            mode: first?.mode ?? "strict",
          },
          right: { backend: "cobol-gnucobol", mode: "strict" },
        },
      ),
    ).toThrow(/cobol-gnucobol/);
  });

  /// The ISA comes out of the machine record the campaign wrote, never out of the
  /// filename. An absolute timing never crosses an ISA, and the site keeps two
  /// campaigns apart on this field — so it had better be the machine's own word.
  it("reports the ISA of every campaign, from inside the file", () => {
    for (const file of campaigns()) {
      const raw = readFileSync(resolve(FIXTURES, file), "utf8");
      const analysis = analysisSchema.parse(analyze(raw, { include_warmup: false }));

      expect(analysis.arch).not.toBe("");
      // The name is a convenience for a human reading `ls`. If it and the header
      // ever disagree, the header is right -- but they should not disagree, and a
      // campaign filed under the wrong ISA is worth catching here.
      expect(file).toBe(`${analysis.arch}.ndjson`);
    }
  });
});
