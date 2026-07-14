// Which implementation card the address bar is pointing at.
//
// A row of a campaign's results table links to `/workloads/<workload>/#impl-c-gcc-none`
// to answer "what *is* this thing" — and it lands the reader in the middle of thirty
// cards that all look alike. So the one they asked for is marked.
//
// `:target` alone would be the whole answer on a plain static site, and the CSS keeps it
// as the no-JavaScript fallback. But this site swaps pages with the view-transition
// router, which navigates by `pushState` — and a `pushState` does not recompute the
// document's target element. A reader arriving from a table would land on the right card
// with nothing marking it. So the mark is put on explicitly, and this is the code that
// does it.

const MARK = "found";

/**
 * Mark the card the URL names, and unmark whatever was marked before.
 *
 * The hash is a URL somebody can type: it is looked up by id, never interpolated into a
 * selector. A hash that names nothing, or something that is not a card, marks nothing —
 * quietly, because a stale link is not an error worth shouting about.
 */
export function markTarget(): HTMLElement | null {
  for (const card of document.querySelectorAll(`.impl.${MARK}`)) {
    card.classList.remove(MARK);
  }

  const id = window.location.hash.slice(1);
  if (id === "") {
    return null;
  }

  const card = document.getElementById(id);
  if (card === null || !card.classList.contains("impl")) {
    return null;
  }

  card.classList.add(MARK);
  return card;
}

/**
 * Keep the mark in step with the address bar, for the life of the tab.
 *
 * `astro:page-load` fires on the first load *and* after every swap the router makes;
 * `hashchange` covers a second click on a link to another card, where the document does
 * not change at all and no navigation event fires.
 */
export function watchTarget(): void {
  const mark = () => {
    // The router restores a scroll position of its own. The anchor is what the reader
    // actually asked for, so it wins.
    markTarget()?.scrollIntoView({ block: "start" });
  };
  document.addEventListener("astro:page-load", mark);
  window.addEventListener("hashchange", mark);
}
