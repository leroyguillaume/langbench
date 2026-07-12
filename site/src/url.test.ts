// The query string is an I/O boundary. It is validated, never trusted.

import { beforeEach, describe, expect, it } from "vitest";
import {
  type CompareState,
  compareHref,
  NO_FILTERS,
  type ResultsState,
  readCompare,
  readResults,
  writeCompare,
  writeResults,
} from "./url";

function at(query: string): void {
  window.history.replaceState(null, "", `/${query}`);
}

describe("reading the results view out of the URL", () => {
  beforeEach(() => at(""));

  it("defaults to every mode, fastest first — the order the report ranks in", () => {
    const state = readResults();
    expect(state.arch).toBeNull();
    expect(state.filters).toStrictEqual(NO_FILTERS);
    expect(state.sort).toStrictEqual({ key: "run", descending: false });
    expect(state.includeWarmup).toBe(false);
  });

  it("keeps the modes it recognizes and drops the ones it does not", () => {
    at("?mode=fast,rubbish,strict");
    // Canonical order, not the order they were typed: the colour of a series is
    // its slot, and the slot is fixed.
    expect(readResults().filters.modes).toStrictEqual(["strict", "fast"]);
  });

  it("falls back to the default sort rather than trusting a key it has no column for", () => {
    at("?sort=drop_table&desc=1");
    expect(readResults().sort).toStrictEqual({ key: "run", descending: false });
  });

  it("sorts on a column of the triple, because that is what names a row", () => {
    at("?sort=interpreter&desc=1");
    expect(readResults().sort).toStrictEqual({
      key: "interpreter",
      descending: true,
    });
  });

  it("never lands on an empty mode list, whatever the query string says", () => {
    at("?mode=nonsense");
    expect(readResults().filters.modes).toStrictEqual(["strict", "fma", "fast"]);
  });

  it("round-trips a view, so a filtered table is a link somebody else can open", () => {
    const state: ResultsState = {
      arch: "x86_64",
      algo: "mandelbrot",
      includeWarmup: true,
      filters: {
        language: "python",
        compiler: "cython",
        interpreter: "cpython",
        search: "cy",
        modes: ["strict"],
      },
      sort: { key: "binary", descending: true },
    };
    writeResults(state);
    expect(readResults()).toStrictEqual(state);
  });

  // "Every compiler" and "the ones with no compiler" are two different questions,
  // and the second one has an answer: every ahead-of-time backend in the table.
  it("round-trips a filter on an *absent* half of the triple", () => {
    const state: ResultsState = {
      arch: null,
      algo: null,
      includeWarmup: false,
      filters: { ...NO_FILTERS, interpreter: "-" },
      sort: { key: "run", descending: false },
    };
    writeResults(state);
    expect(window.location.search).toBe("?interpreter=-");
    expect(readResults().filters.interpreter).toBe("-");
  });

  it("leaves the default view out of the URL entirely", () => {
    writeResults({
      arch: null,
      algo: null,
      includeWarmup: false,
      filters: NO_FILTERS,
      sort: { key: "run", descending: false },
    });
    expect(window.location.search).toBe("");
  });
});

// A head-to-head is the sharpest claim this site makes, and a claim nobody can link
// to is a claim nobody can check.
describe("reading the head-to-head out of the URL", () => {
  beforeEach(() => at(""));

  it("carries the two rows, spelled as the triple and never as a slug", () => {
    at("?a=c/gcc/-/strict&b=python/-/cpython/fast");
    const state = readCompare();
    expect(state.left).toBe("c/gcc/-/strict");
    expect(state.right).toBe("python/-/cpython/fast");
  });

  it("round-trips a pair", () => {
    const state: CompareState = {
      arch: "aarch64",
      algo: "mandelbrot",
      includeWarmup: false,
      left: "java/native-image/-/strict",
      right: "java/javac/openjdk/strict",
    };
    writeCompare(state);
    expect(readCompare()).toStrictEqual(state);
  });
});

// The filters narrow a table; a pair is not a table. But the campaign does travel:
// a "Compare" link that quietly switched architecture would invite exactly the
// comparison `METHODOLOGY.md#the-isa-rule` forbids.
describe("the link from the results to the head-to-head", () => {
  it("carries the ISA and the algorithm, and nothing else", () => {
    const href = compareHref({
      arch: "aarch64",
      algo: "mandelbrot",
      includeWarmup: false,
    });
    expect(href).toContain("compare/");
    expect(href).toContain("arch=aarch64");
    expect(href).toContain("algo=mandelbrot");
    expect(href).not.toContain("language=");
  });

  it("says nothing when there is nothing to say", () => {
    expect(compareHref({ arch: null, algo: null, includeWarmup: false })).not.toContain("?");
  });
});
