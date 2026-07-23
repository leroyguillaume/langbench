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

  it("defaults to every mode, fastest first — the order the table ranks in", () => {
    const state = readResults();
    expect(state.filters).toStrictEqual(NO_FILTERS);
    expect(state.sort).toStrictEqual({ key: "run", descending: false });
    expect(state.includeWarmup).toBe(false);
  });

  it("keeps the modes it recognizes and drops the ones it does not", () => {
    at("?mode=native,rubbish,baseline");
    // Canonical order, not the order they were typed: the colour of a series is
    // its slot, and the slot is fixed.
    expect(readResults().filters.modes).toStrictEqual(["baseline", "native"]);
  });

  // The floating-point modes this axis replaced. They are not aliases of anything —
  // `fma` and `fast` computed a *different number* — so a bookmarked link naming one
  // must fall back to every mode, never quietly reinterpret it as a row that ran.
  it("drops the floating-point modes a stale link still asks for", () => {
    at("?mode=strict,fma,fast");
    expect(readResults().filters.modes).toStrictEqual(["baseline", "native"]);
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
    expect(readResults().filters.modes).toStrictEqual(["baseline", "native"]);
  });

  it("round-trips a view, so a filtered table is a link somebody else can open", () => {
    const state: ResultsState = {
      includeWarmup: true,
      filters: {
        language: "python",
        compiler: "cython",
        interpreter: "cpython",
        search: "cy",
        modes: ["baseline"],
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
      includeWarmup: false,
      filters: NO_FILTERS,
      sort: { key: "run", descending: false },
    });
    expect(window.location.search).toBe("");
  });

  // The campaign is the *path* — `/workloads/mandelbrot/x86_64/`. A query string that
  // also named one would be a second answer to a question the URL has already
  // answered, and the two would be free to contradict each other.
  it("never puts the campaign in the query string", () => {
    at("?architecture=aarch64&workload=nbody&q=gcc");
    const state = readResults();
    expect(state).not.toHaveProperty("architecture");
    expect(state).not.toHaveProperty("workload");

    writeResults(state);
    expect(window.location.search).toBe("?q=gcc");
  });
});

// A head-to-head is the sharpest claim this site makes, and a claim nobody can link
// to is a claim nobody can check.
describe("reading the head-to-head out of the URL", () => {
  beforeEach(() => at(""));

  it("carries the two rows, spelled as the triple and never as a slug", () => {
    at("?a=c/gcc/-/baseline&b=python/-/cpython/native");
    const state = readCompare();
    expect(state.left).toBe("c/gcc/-/baseline");
    expect(state.right).toBe("python/-/cpython/native");
  });

  it("round-trips a pair", () => {
    const state: CompareState = {
      architecture: "aarch64",
      workload: "mandelbrot",
      includeWarmup: false,
      left: "java/native-image/-/baseline",
      right: "java/javac/openjdk/native",
    };
    writeCompare(state);
    expect(readCompare()).toStrictEqual(state);
  });
});

// The filters narrow a table; a pair is not a table. But the campaign does travel:
// a "Compare" link that quietly switched architecture would invite exactly the
// comparison the architecture rule forbids. The campaign is the results page's *path*,
// so this is where it re-enters a query string — the head-to-head is the one page that
// may be handed two campaigns.
describe("the link from the results to the head-to-head", () => {
  it("carries the architecture and the workload, and nothing else", () => {
    const href = compareHref({
      architecture: "aarch64",
      workload: "mandelbrot",
      includeWarmup: false,
    });
    expect(href).toContain("compare/");
    expect(href).toContain("architecture=aarch64");
    expect(href).toContain("workload=mandelbrot");
    expect(href).not.toContain("language=");
  });

  it("says nothing when there is nothing to say", () => {
    expect(compareHref({ architecture: null, workload: null, includeWarmup: false })).not.toContain(
      "?",
    );
  });
});
