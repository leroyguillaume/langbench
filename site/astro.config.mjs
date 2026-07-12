// Static output, because the host is a file server.
//
// GitHub Pages serves a project site under `/<repo>/` and has no server-side
// anything: no rewrite, no redirect, no content negotiation. So every route is
// prerendered to a real `.html` on disk -- `/compare/` exists, and a deep link, a
// refresh and a shared URL all work without the `404.html` fallback a client-side
// router would need. `base` is a build-time input rather than a constant, so the
// same source builds for Pages, for a custom domain, and for `astro dev` at `/`.
//
// The pages are shells. Everything that reads a campaign is a React island marked
// `client:only`, because it needs the WebAssembly and the WebAssembly needs a
// browser -- there is no campaign to analyze at build time, and a page that
// pretended otherwise would ship a chart of numbers nobody measured.

import react from "@astrojs/react";
import { defineConfig } from "astro/config";

export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  output: "static",
  integrations: [react()],
  markdown: {
    // METHODOLOGY.md is prose, and it is long. Headings get anchors so the rules
    // scattered across `CLAUDE.md` can keep linking to the section that justifies
    // them -- the links in the repository and the links on the site are the same
    // links.
    shikiConfig: {
      themes: {
        light: "github-light",
        dark: "github-dark",
      },
    },
  },
});
