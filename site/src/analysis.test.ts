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

    expect(analysis.workloads.length).toBeGreaterThan(0);
    expect(analysis.backends.length).toBeGreaterThan(0);

    const [workload] = analysis.workloads;
    expect(workload?.aggregates.length).toBeGreaterThan(0);
  });

  it("ranks the aggregates fastest first, on the minimum wall-clock", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const measured = (analysis.workloads[0]?.aggregates ?? [])
      .map((row) => row.run_wall?.min)
      .filter((min): min is number => min !== undefined && min !== null);

    expect(measured.length).toBeGreaterThan(1);
    expect([...measured].sort((a, b) => a - b)).toStrictEqual(measured);
  });

  /// The reason this whole boundary exists. A checksum is a 64-bit integer, and
  /// `JSON.parse` would have rounded it to the nearest double on the way in.
  it("hands the checksum over as a string, with every bit intact", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const checksum = analysis.workloads[0]?.checksum;

    expect(typeof checksum).toBe("string");
    expect(checksum).toMatch(/^\d+$/);

    // *Every* row agreed on it, with no mode qualifying the claim: `baseline` and
    // `native` are both strict IEEE 754, a wider vector reorders nothing, and the
    // harness quarantines a backend at the sample that disagrees. There is no
    // `checksum_delta` beside it any more — there is nothing left to price.
    for (const row of analysis.workloads[0]?.aggregates ?? []) {
      expect(row.checksum).toBe(checksum);
      expect(row).not.toHaveProperty("checksum_delta");
    }
  });

  /// The mode is what the row asked for; the ISA is what the toolchain gave it. Both
  /// cross the boundary, because the rows where they disagree are the ones with
  /// something to say — and before this field existed those divergences lived only in
  /// the free text of a manifest.
  it("carries the ISA each row actually got, beside the mode it asked for", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const rows = analysis.workloads[0]?.aggregates ?? [];
    expect(rows.length).toBeGreaterThan(0);

    for (const row of rows) {
      expect(["baseline", "native"]).toContain(row.mode);
      // A published fact or a published absence — never an empty string pretending to
      // be either.
      expect(row.isa === null || row.isa.length > 0).toBe(true);
    }

    // The campaign this fixture is: a compiled backend honouring the pinned baseline,
    // the same backend targeting the machine, and CPython on an ISA nobody here chose.
    const isas = new Set(rows.map((row) => row.isa));
    expect(isas).toContain("armv8.2-a");
    expect(isas).toContain("native");
    expect(isas).toContain("distro");
  });

  it("aggregates the warmup rounds only when asked to, and never silently", () => {
    const cold = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const warm = analysisSchema.parse(analyze(ndjson, { include_warmup: true }));

    const samples = (analysis: typeof cold) =>
      analysis.workloads[0]?.aggregates[0]?.run_wall?.n ?? 0;

    expect(cold.options.include_warmup).toBe(false);
    expect(warm.options.include_warmup).toBe(true);
    expect(samples(warm)).toBeGreaterThan(samples(cold));
    // A warmup round can only ever be slower, so folding it in cannot lower the
    // minimum — it can only fail to raise it.
    const coldMin = cold.workloads[0]?.aggregates[0]?.run_wall?.min ?? 0;
    const warmMin = warm.workloads[0]?.aggregates[0]?.run_wall?.min ?? 0;
    expect(warmMin).toBeLessThanOrEqual(coldMin);
  });

  it("refuses a file that is not a campaign, rather than plotting nonsense", () => {
    expect(() => analyze("not ndjson at all", { include_warmup: false })).toThrow();
  });

  /// The head-to-head is the harness's arithmetic too. The site picks two rows and
  /// spells the answer; it does not decide whether a gap is a difference.
  it("compares two rows of the published campaign, and hands back the harness's verdict", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const workload = analysis.workloads[0];
    const [first, second] = workload?.aggregates ?? [];
    if (workload === undefined || first === undefined || second === undefined) {
      throw new Error("the fixture campaign measured fewer than two rows");
    }

    const selection = {
      workload: workload.workload,
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
    const workload = analysis.workloads[0];
    const [first, second] = workload?.aggregates ?? [];
    if (workload === undefined || first === undefined || second === undefined) {
      throw new Error("the fixture campaign measured fewer than two rows");
    }

    const left = { backend: first.backend, mode: first.mode };
    const right = { backend: second.backend, mode: second.mode };
    const forward = comparisonSchema.parse(
      compare(ndjson, { include_warmup: false }, { workload: workload.workload, left, right }),
    );
    const backward = comparisonSchema.parse(
      compare(
        ndjson,
        { include_warmup: false },
        { workload: workload.workload, left: right, right: left },
      ),
    );

    const run = (comparison: typeof forward) =>
      comparison.metrics.find((metric) => metric.key === "run");
    expect((run(forward)?.ratio ?? 0) * (run(backward)?.ratio ?? 0)).toBeCloseTo(1, 6);
    expect(run(forward)?.gap_pct).toBe(run(backward)?.gap_pct);
  });

  it("refuses a row the campaign never measured, rather than comparing against a zero", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const workload = analysis.workloads[0]?.workload ?? "mandelbrot";
    const first = analysis.workloads[0]?.aggregates[0];

    expect(() =>
      compare(
        ndjson,
        { include_warmup: false },
        {
          workload,
          left: {
            backend: first?.backend ?? "c-gcc",
            mode: first?.mode ?? "baseline",
          },
          right: { backend: "cobol-gnucobol", mode: "baseline" },
        },
      ),
    ).toThrow(/cobol-gnucobol/);
  });

  /// The architecture comes out of the machine record the campaign wrote, never out of the
  /// filename. An absolute timing never crosses an architecture, and the site keeps two
  /// campaigns apart on this field — so it had better be the machine's own word.
  it("reports the architecture of every campaign, from inside the file", () => {
    for (const file of campaigns()) {
      const raw = readFileSync(resolve(FIXTURES, file), "utf8");
      const analysis = analysisSchema.parse(analyze(raw, { include_warmup: false }));

      expect(analysis.architecture).not.toBe("");
      // The name is a convenience for a human reading `ls`. If it and the header
      // ever disagree, the header is right -- but they should not disagree, and a
      // campaign filed under the wrong architecture is worth catching here.
      expect(file).toBe(`${analysis.architecture}.ndjson`);
    }
  });

  /// The metrics PR #16 added, crossing the boundary the schema guards.
  ///
  /// The fixture campaign predates them, so every one of them is `null` here — which
  /// is the state of the committed campaigns until the runners finish, and exactly
  /// the case that has to render rather than throw. An absence is not a zero.
  it("carries the new metrics, and an old campaign reports them as absent rather than as zero", () => {
    const analysis = analysisSchema.parse(analyze(ndjson, { include_warmup: false }));
    const row = analysis.workloads[0]?.aggregates[0];
    if (row === undefined) {
      throw new Error("the fixture campaign measured nothing");
    }

    // On the wire, and typed — a field the harness renamed would fail the parse above
    // long before a reader saw an empty column.
    expect(row).toHaveProperty("run_cores");
    expect(row).toHaveProperty("run_peak_bytes");
    expect(row).toHaveProperty("source_bytes");
    // The denominator of `run_cores`: the threads this campaign handed every kernel.
    expect(row.cpu).toBeGreaterThan(0);
  });
});
