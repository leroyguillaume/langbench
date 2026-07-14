// A reader who clicked a row of a results table has to *see* which card answered them.

import { beforeEach, describe, expect, it } from "vitest";
import { markTarget } from "./target";

function cards(): void {
  document.body.innerHTML = `
    <article class="impl" id="impl-c-gcc-none"></article>
    <article class="impl" id="impl-python-none-cpython"></article>
    <section class="card" id="columns"></section>
  `;
}

function at(hash: string): void {
  window.history.replaceState(null, "", `/workloads/mandelbrot/${hash}`);
}

describe("the card the address bar names", () => {
  beforeEach(cards);

  it("marks the one the hash names, and nothing else", () => {
    at("#impl-python-none-cpython");
    expect(markTarget()?.id).toBe("impl-python-none-cpython");

    const marked = document.querySelectorAll(".impl.found");
    expect(marked).toHaveLength(1);
    expect(marked[0]?.id).toBe("impl-python-none-cpython");
  });

  // Otherwise a reader following two links in a row ends up with two cards claiming to
  // be the one they asked for, which is worse than marking none.
  it("moves the mark rather than adding a second one", () => {
    at("#impl-c-gcc-none");
    markTarget();
    at("#impl-python-none-cpython");
    markTarget();

    expect(document.querySelectorAll(".impl.found")).toHaveLength(1);
    expect(document.querySelector(".impl.found")?.id).toBe("impl-python-none-cpython");
  });

  it("marks nothing when the URL names nothing", () => {
    at("");
    expect(markTarget()).toBeNull();
    expect(document.querySelectorAll(".impl.found")).toHaveLength(0);
  });

  // A stale link, or a hash that points at something that is not a card — the column
  // reference, say, which is a legitimate anchor on another page. Marking nothing is the
  // answer; an exception is not.
  it("marks nothing when the hash names something that is not an implementation", () => {
    at("#columns");
    expect(markTarget()).toBeNull();

    at("#impl-fortran-none-none");
    expect(markTarget()).toBeNull();
    expect(document.querySelectorAll(".impl.found")).toHaveLength(0);
  });
});
