// The triple is what names a row. The slug is what the WASM picks it by, and the
// two must never be confused — `java-native-image` reads as "java, native" and is
// in fact java, compiled by `native-image`, with no interpreter at all.

import { describe, expect, it } from "vitest";
import type { Aggregate } from "./analysis";
import { findByKey, identityKey, label, toolchain } from "./identity";

const aggregate = (
  language: string,
  compiler: string | null,
  interpreter: string | null,
): Aggregate => ({
  algo: "mandelbrot",
  backend: "irrelevant",
  backend_id: "irrelevant",
  language,
  compiler,
  interpreter,
  mode: "strict",
  run_wall: null,
  run_elapsed: null,
  run_startup: null,
  run_cpu_usec: null,
  build_elapsed: null,
  binary_bytes: null,
  binary_stripped_bytes: null,
  text_bytes: null,
  checksum: null,
  checksum_delta: null,
});

describe("naming an implementation", () => {
  it("names the toolchain that ran it, and mentions only the halves that exist", () => {
    expect(toolchain({ language: "c", compiler: "gcc", interpreter: null })).toBe("gcc");
    expect(toolchain({ language: "python", compiler: null, interpreter: "cpython" })).toBe(
      "cpython",
    );
    expect(
      toolchain({
        language: "python",
        compiler: "cython",
        interpreter: "cpython",
      }),
    ).toBe("cython + cpython");
    // The one the slug got wrong: a compiler, and no interpreter at all.
    expect(
      toolchain({
        language: "java",
        compiler: "native-image",
        interpreter: null,
      }),
    ).toBe("native-image");
  });

  it("labels a row the way the report's columns read", () => {
    expect(label({ language: "java", compiler: "javac", interpreter: "openjdk" })).toBe(
      "java · javac + openjdk",
    );
  });
});

describe("the key a link carries", () => {
  it("spells the four fields out, so a shared URL can be read by a human", () => {
    expect(
      identityKey({
        language: "java",
        compiler: "native-image",
        interpreter: null,
        mode: "strict",
      }),
    ).toBe("java/native-image/-/strict");
  });

  it("finds the row it points at", () => {
    const rows = [aggregate("c", "gcc", null), aggregate("java", "native-image", null)];
    expect(findByKey(rows, "java/native-image/-/strict")?.language).toBe("java");
  });

  // A key arrives from a query string somebody may have typed, or bookmarked before
  // a backend was renamed. It is resolved against the campaign, never trusted.
  it("drops a triple this campaign never measured rather than passing it to the harness", () => {
    const rows = [aggregate("c", "gcc", null)];
    expect(findByKey(rows, "c/clang/-/strict")).toBeNull();
    expect(findByKey(rows, "c/gcc/-/turbo")).toBeNull();
    expect(findByKey(rows, "nonsense")).toBeNull();
    expect(findByKey(rows, null)).toBeNull();
  });
});
