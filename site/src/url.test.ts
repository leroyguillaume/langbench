// The query string is an I/O boundary. It is validated, never trusted.

import { beforeEach, describe, expect, it } from "vitest";
import { readUrl, writeUrl } from "./url";

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
    const state = {
      arch: "x86_64",
      algo: "mandelbrot",
      language: "c",
      search: "gcc",
      modes: ["strict"] as const,
      includeWarmup: true,
      sort: { key: "binary", descending: true },
    };
    writeUrl({ ...state, modes: [...state.modes], sort: { key: "binary", descending: true } });
    expect(readUrl()).toStrictEqual({
      arch: "x86_64",
      algo: "mandelbrot",
      language: "c",
      search: "gcc",
      modes: ["strict"],
      includeWarmup: true,
      sort: { key: "binary", descending: true },
    });
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
    });
    expect(window.location.search).toBe("");
  });
});
