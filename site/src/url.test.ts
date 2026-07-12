// The query string is an I/O boundary. It is validated, never trusted.

import { beforeEach, describe, expect, it } from "vitest";
import { readUrl, type UrlState, writeUrl } from "./url";

function at(query: string): void {
  window.history.replaceState(null, "", `/${query}`);
}

describe("reading the view out of the URL", () => {
  beforeEach(() => at(""));

  it("defaults to every mode, fastest first — the order the report ranks in", () => {
    const state = readUrl();
    expect(state.arch).toBeNull();
    expect(state.modes).toStrictEqual(["strict", "fma", "fast"]);
    expect(state.sort).toStrictEqual({ key: "run", descending: false });
    expect(state.includeWarmup).toBe(false);
  });

  it("keeps the modes it recognizes and drops the ones it does not", () => {
    at("?mode=fast,rubbish,strict");
    // Canonical order, not the order they were typed: the colour of a series is
    // its slot, and the slot is fixed.
    expect(readUrl().modes).toStrictEqual(["strict", "fast"]);
  });

  it("falls back to the default sort rather than trusting a key it has no column for", () => {
    at("?sort=drop_table&desc=1");
    expect(readUrl().sort).toStrictEqual({ key: "run", descending: false });
  });

  it("never lands on an empty mode list, whatever the query string says", () => {
    at("?mode=nonsense");
    expect(readUrl().modes).toStrictEqual(["strict", "fma", "fast"]);
  });

  it("round-trips a view, so a filtered table is a link somebody else can open", () => {
    const state: UrlState = {
      arch: "x86_64",
      algo: "mandelbrot",
      language: "c",
      search: "gcc",
      modes: ["strict"],
      includeWarmup: true,
      sort: { key: "binary", descending: true },
      compareLeft: { backend: "c-gcc", mode: "strict" },
      compareRight: { backend: "rust-llvm", mode: "fast" },
    };
    writeUrl(state);
    expect(readUrl()).toStrictEqual(state);
  });

  it("leaves the default view out of the URL entirely", () => {
    writeUrl({
      arch: null,
      algo: null,
      language: null,
      search: "",
      modes: ["strict", "fma", "fast"],
      includeWarmup: false,
      sort: { key: "run", descending: false },
      compareLeft: null,
      compareRight: null,
    });
    expect(window.location.search).toBe("");
  });

  // A head-to-head is the sharpest claim this site makes, and a claim nobody can
  // link to is a claim nobody can check.
  it("carries the two rows of a head-to-head, so a comparison is a link", () => {
    at("?a=c-gcc:strict&b=python-cpython:fast");
    expect(readUrl().compareLeft).toStrictEqual({ backend: "c-gcc", mode: "strict" });
    expect(readUrl().compareRight).toStrictEqual({ backend: "python-cpython", mode: "fast" });
  });

  it("drops a row whose mode is not one of the three, rather than passing it to the harness", () => {
    at("?a=c-gcc:turbo&b=:strict");
    expect(readUrl().compareLeft).toBeNull();
    expect(readUrl().compareRight).toBeNull();
  });
});
