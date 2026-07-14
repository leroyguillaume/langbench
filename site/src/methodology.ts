// The methodology, as a set of pages.
//
// The prose lives in `src/content/methodology/`, **not** in `src/pages/`, and the
// distinction is load-bearing rather than tidy. A Markdown *page* carries its own
// layout, and importing its `Content` somewhere else drags that layout in with it —
// which is how the column reference, slotted under a campaign's table, once brought a
// second `<html>` document with it. Content is content; a route is a route. So these
// files are prose with frontmatter, `[...slug].astro` gives them their routes, and the
// `/data/` page imports `data.md` as what it is: a fragment.
//
// The pages are globbed, never listed. Adding one is adding a file — the route, the
// sidebar entry and the index all follow from its frontmatter.

import type { MarkdownInstance } from "astro";

export interface Frontmatter {
  title: string;
  /** Where it sits in the reading order. The section is meant to be read in it. */
  order: number;
  summary: string;
}

export interface Page {
  /** `index` is the section's landing page, served at `/methodology/`. */
  slug: string | undefined;
  frontmatter: Frontmatter;
  entry: MarkdownInstance<Frontmatter>;
}

const modules = import.meta.glob<MarkdownInstance<Frontmatter>>("./content/methodology/*.md", {
  eager: true,
});

/** Every methodology page, in the order its frontmatter asks to be read in. */
export const pages: Page[] = Object.entries(modules)
  .map(([path, entry]) => {
    const name = path.split("/").pop()?.replace(/\.md$/, "") ?? "";
    return {
      slug: name === "index" ? undefined : name,
      frontmatter: entry.frontmatter,
      entry,
    };
  })
  .sort((left, right) => left.frontmatter.order - right.frontmatter.order);

/** Its route, under whatever `base` the site is served from. */
export function href(base: string, page: Page): string {
  return page.slug === undefined ? `${base}methodology/` : `${base}methodology/${page.slug}/`;
}
