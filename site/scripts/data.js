// Copies the campaign the site publishes into `public/`, where Vite serves it.
//
// The site's data file is the campaign's `samples.ndjson`, byte for byte. There
// is no export step and no intermediate format: the raw samples are the only
// thing a run writes and the only thing that cannot be recomputed, so they are
// what gets published. Everything the site shows, it derives — with the harness's
// own code, compiled to WebAssembly. See `METHODOLOGY.md#sampling`.
//
// A config file the tooling requires in JS: the site's own source is TypeScript.

import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const source = resolve(here, "..", "..", "samples.ndjson");
const target = resolve(here, "..", "public", "data", "samples.ndjson");

if (!existsSync(source)) {
  // Not a warning to be scrolled past: a site built without a campaign is a site
  // that renders an error page. Fail here, where the message is legible.
  console.error(
    `no campaign to publish: ${source} does not exist.\n` +
      "Run a campaign; `samples.ndjson` at the repo root is where it lands by default:\n" +
      "  langbench run",
  );
  process.exit(1);
}

mkdirSync(dirname(target), { recursive: true });
copyFileSync(source, target);
console.log(`published ${source}`);
