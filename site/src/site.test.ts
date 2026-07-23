// A description is prose, and its only structure is its blank lines. `runIn` adds one
// affordance on top of that — a short opening label rendered as a bold run-in heading —
// and the point of these tests is that it stays narrow: it must lift a real label out of
// a paragraph and leave ordinary prose untouched.

import { describe, expect, it } from "vitest";
import { paragraphs, runIn } from "./site";

describe("paragraphs", () => {
  it("splits on blank lines and trims", () => {
    expect(paragraphs("one\n\n  two  \n\n\n three ")).toEqual(["one", "two", "three"]);
  });

  it("drops empty stretches rather than emitting empty paragraphs", () => {
    expect(paragraphs("\n\n\nonly\n\n")).toEqual(["only"]);
  });
});

describe("runIn", () => {
  it("lifts a short opening label into a bold run-in heading", () => {
    expect(runIn("What it puts under the light. A tight scalar floating-point loop.")).toEqual({
      lead: "What it puts under the light.",
      body: "A tight scalar floating-point loop.",
    });
  });

  it("lifts the second label the mandelbrot description uses", () => {
    expect(runIn("What it says nothing about. There is no allocation in the hot loop.")).toEqual({
      lead: "What it says nothing about.",
      body: "There is no allocation in the hot loop.",
    });
  });

  it("leaves ordinary prose alone when the opening sentence is long", () => {
    const prose =
      "Escape-time Mandelbrot over a square grid iterates every pixel until it escapes.";
    expect(runIn(prose)).toEqual({ body: prose });
  });

  it("does not treat an opening clause with internal punctuation as a label", () => {
    const prose = "Escape-time Mandelbrot, over a square grid. The rest follows.";
    expect(runIn(prose)).toEqual({ body: prose });
  });

  it("leaves a lone label with no body alone", () => {
    expect(runIn("What it puts under the light.")).toEqual({
      body: "What it puts under the light.",
    });
  });
});
