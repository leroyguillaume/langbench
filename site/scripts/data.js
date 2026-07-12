// Copies the campaigns the site publishes into `public/`, where Vite serves them.
//
// The site's data files are the campaigns in `samples/` — or wherever `SAMPLES_DIR`
// points — byte for byte. There is no export step and no intermediate format: the
// raw samples are the only thing a run writes and the only thing that cannot be
// recomputed, so they are what gets published. Everything the site shows, it
// derives — with the harness's own code, compiled to WebAssembly.
// See `METHODOLOGY.md#sampling`.
//
// One campaign per ISA, because **an absolute timing never crosses an ISA**
// (`METHODOLOGY.md#the-isa-rule`). The architecture in the filename is a
// convenience for a human reading `ls`; it is *not* what the site keys on. The
// ISA is read out of the machine record inside each file, by the WASM — a
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

let entries;
try {
  entries = readdirSync(source);
} catch (error) {
  // A pointed-at directory that does not exist is a typo, not an empty campaign
  // set. Say which path, and say who chose it.
  console.error(
    `cannot read the campaign directory ${source}: ${error.message}\n` + `It came from ${chosen}.`,
  );
  process.exit(1);
}

const campaigns = entries.filter((name) => CAMPAIGN.test(name)).sort();

if (campaigns.length === 0) {
  // Not a warning to be scrolled past: a site built without a campaign is a site
  // that renders an error page. Fail here, where the message is legible.
  console.error(
    `no campaign to publish: no .ndjson in ${source} (${chosen}).\n` +
      "Run one, and name it after the ISA it ran on:\n" +
      "  langbench run --output samples/x86_64.ndjson\n" +
      "Or point the site at a campaign you already have:\n" +
      "  SAMPLES_DIR=samples.local npm run dev",
  );
  process.exit(1);
}

mkdirSync(target, { recursive: true });
for (const campaign of campaigns) {
  copyFileSync(join(source, campaign), join(target, campaign));
}

// The index the site fetches first. Filenames only — every fact about a campaign
// (its ISA, its host, its date) lives in the campaign itself.
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
const methodology = resolve(root, "METHODOLOGY.md");
const generated = resolve(here, "..", "src", "generated");
mkdirSync(generated, { recursive: true });
try {
  copyFileSync(methodology, join(generated, "methodology.md"));
} catch (error) {
  console.error(`cannot read ${methodology}: ${error.message}`);
  process.exit(1);
}
console.log(`published METHODOLOGY.md from ${methodology}`);
