// Copies the campaigns the site publishes into `public/`, where Vite serves them.
//
// The site's data files are the campaigns in `samples/` — or wherever `SAMPLES_DIR`
// points — byte for byte. There is no export step and no intermediate format: the
// raw samples are the only thing a run writes and the only thing that cannot be
// recomputed, so they are what gets published. Everything the site shows, it
// derives — with the harness's own code, compiled to WebAssembly.
// See `METHODOLOGY.md#sampling`.
//
// One campaign per (workload, architecture), because **an absolute timing never crosses an
// architecture**
// (`METHODOLOGY.md#the-architecture-rule`). The architecture in the filename is a
// convenience for a human reading `ls`; it is *not* what the site keys on. The
// architecture is read out of the machine record inside each file, by the WASM — a
// filename is a label somebody typed, and the header is what the machine said
// about itself.
//
// A config file the tooling requires in JS: the site's own source is TypeScript.

import { copyFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");
const target = resolve(here, "..", "public", "data");

const DEFAULT_SOURCE = "samples";

// Where the campaigns are read from: `samples/`, the campaigns the repository
// publishes, unless `SAMPLES_DIR` says otherwise — `samples.local/` to look at a
// campaign you ran here and are not committing. A relative value is relative to the
// repository root, not to `site/`, so it reads the same as the `--output` the run
// was given; an absolute path is taken as it is.
const override = process.env.SAMPLES_DIR;
const source = resolve(root, override ?? DEFAULT_SOURCE);
const chosen =
  override === undefined ? `${DEFAULT_SOURCE}/, the default` : `SAMPLES_DIR=${override}`;

const CAMPAIGN = /\.ndjson$/;

// `samples/<workload>/<architecture>.ndjson`, and also a plain `*.ndjson` at the top
// level — `samples.local/` holds one file, written by hand with `--output`.
//
// The directory name is a convenience for a human reading `ls`, exactly like the
// architecture in the filename: **neither is what the site keys on**. Both the
// workload and the architecture are read out of the campaign's own header, by the
// WASM. A path is a label somebody typed; the header is what the run recorded.
function campaignsIn(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch (error) {
    // A pointed-at directory that does not exist is a typo, not an empty campaign
    // set. Say which path, and say who chose it.
    console.error(
      `cannot read the campaign directory ${dir}: ${error.message}\n` + `It came from ${chosen}.`,
    );
    process.exit(1);
  }

  const found = [];
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    if (entry.isDirectory()) {
      found.push(...campaignsIn(join(dir, entry.name)).map((name) => join(entry.name, name)));
    } else if (CAMPAIGN.test(entry.name)) {
      found.push(entry.name);
    }
  }
  return found;
}

const campaigns = campaignsIn(source);

if (campaigns.length === 0) {
  // Not a warning to be scrolled past: a site built without a campaign is a site
  // that renders an error page. Fail here, where the message is legible.
  console.error(
    `no campaign to publish: no .ndjson in ${source} (${chosen}).\n` +
      "Run one:\n" +
      "  langbench workload run mandelbrot --output samples/mandelbrot/x86_64.ndjson\n" +
      "Or point the site at a campaign you already have:\n" +
      "  SAMPLES_DIR=samples.local npm run dev",
  );
  process.exit(1);
}

for (const campaign of campaigns) {
  const destination = join(target, campaign);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(join(source, campaign), destination);
}

// The index the site fetches first. Filenames only — every fact about a campaign
// (its workload, its architecture, its host, its date) lives in the campaign itself.
writeFileSync(join(target, "campaigns.json"), `${JSON.stringify(campaigns, null, 2)}\n`);
console.log(`published ${campaigns.length} campaign(s) from ${chosen}: ${campaigns.join(", ")}`);

// METHODOLOGY.md, copied rather than rewritten.
//
// It is the document every rule in this repository links to when it looks like
// excessive caution, and the site publishes it for the same reason the README
// leads with it: a number nobody can audit is a number nobody should trust. It is
// copied *in*, at build time, because Astro only renders Markdown it can see under
// `src/` — and a second, hand-maintained copy of it on the site would be a
// methodology that drifts from the one the harness is written against, which is
// the failure `bench.schema.json` is generated to avoid.
// The same goes for `docs/columns.md`: what every column of the results table means,
// and how to read a row. `langbench md` interpolates that file into the report and
// the site renders it under the same table — one explanation of why we report the
// minimum and not the average, written once, for a reader who has never opened a
// benchmark before, and improved in one place.
const generated = resolve(here, "..", "src", "generated");
mkdirSync(generated, { recursive: true });

const shared = [
  { from: resolve(root, "METHODOLOGY.md"), to: "methodology.md" },
  { from: resolve(root, "docs", "columns.md"), to: "columns.md" },
];

for (const { from, to } of shared) {
  try {
    copyFileSync(from, join(generated, to));
  } catch (error) {
    console.error(`cannot read ${from}: ${error.message}`);
    process.exit(1);
  }
  console.log(`published ${from}`);
}
