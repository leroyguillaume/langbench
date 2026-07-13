// What the site is built from: the campaigns, and the workloads that declare the work.
//
// Two inputs, and they answer two different questions.
//
// **The campaigns** — `samples/<workload>/<architecture>.ndjson`, or wherever
// `SAMPLES_DIR` points — are copied into `public/` byte for byte. There is no export
// step and no intermediate format: the raw samples are the only thing a run writes
// and the only thing that cannot be recomputed, so they are what gets published.
// Everything the site shows about a campaign, it derives — with the harness's own
// code, compiled to WebAssembly. See `site/src/content/methodology/what-we-record.md`.
//
// **The workloads** come from `langbench workload list --json`: the harness reading
// the manifests, never a YAML parser of our own. They are what the *workload* page
// describes — the work as it is declared today. A campaign page describes the work as
// it *was measured*, which is the snapshot inside the campaign's own header, and the
// two are allowed to differ: editing `workload.yaml` cannot rewrite what a campaign
// from three months ago says it ran.
//
// One campaign per (workload, architecture), because **an absolute timing never
// crosses an architecture** (`flags-and-architectures.md#the-architecture-rule`). The
// path is a convenience for a human reading `ls`; it is *not* what the site keys on.
// The workload and the architecture are read out of the header the run recorded — a
// filename is a label somebody typed.
//
// A config file the tooling requires in JS: the site's own source is TypeScript.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");
const target = resolve(here, "..", "public", "data");
const generated = resolve(here, "..", "src", "generated");

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

function fail(message) {
  console.error(message);
  process.exit(1);
}

/** `samples/<workload>/<architecture>.ndjson`, and a plain `*.ndjson` at the top level. */
function campaignsIn(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch (error) {
    // A pointed-at directory that does not exist is a typo, not an empty campaign
    // set. Say which path, and say who chose it.
    fail(`cannot read the campaign directory ${dir}: ${error.message}\nIt came from ${chosen}.`);
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

/**
 * The facts a *route* needs about a campaign: which workload, which architecture, when,
 * and on what host. Read out of the campaign's own header — never out of its path.
 *
 * This is the one place JavaScript parses a campaign, and it is a build script rather
 * than the browser. It reads the header line and four fields of it; it never touches a
 * checksum, which is a 64-bit integer that `JSON.parse` would silently round past 2^53.
 * (The header spells checksums as strings for that very reason, and the samples under
 * it are never parsed here at all.) In the browser the rule stands unweakened: the
 * campaign is fetched as text and parsed in Rust.
 */
function header(file) {
  const line = readFileSync(join(source, file), "utf8").split("\n", 1)[0];
  let record;
  try {
    record = JSON.parse(line);
  } catch (error) {
    fail(`${file} does not start with a campaign header: ${error.message}`);
  }
  const workload = record.campaign?.workload?.id;
  const architecture = record.machine?.architecture;
  if (typeof workload !== "string" || typeof architecture !== "string") {
    fail(
      `${file} has no workload or no architecture in its header.\n` +
        "A campaign is one machine measuring one workload, and it records both.",
    );
  }
  return {
    file,
    workload,
    architecture,
    timestamp: record.campaign?.timestamp ?? null,
    hostname: record.machine?.hostname ?? null,
  };
}

const files = campaignsIn(source);

if (files.length === 0) {
  // Not a warning to be scrolled past: a site built without a campaign is a site
  // that renders an error page. Fail here, where the message is legible.
  fail(
    `no campaign to publish: no .ndjson in ${source} (${chosen}).\n` +
      "Run one:\n" +
      "  langbench workload run mandelbrot --output samples/mandelbrot/x86_64.ndjson\n" +
      "Or point the site at a campaign you already have:\n" +
      "  SAMPLES_DIR=samples.local npm run dev",
  );
}

for (const file of files) {
  const destination = join(target, file);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(join(source, file), destination);
}

// The index the islands fetch first. Filenames only — every *number* about a campaign
// lives in the campaign, and the WASM is what reads it.
writeFileSync(join(target, "campaigns.json"), `${JSON.stringify(files, null, 2)}\n`);
console.log(`published ${files.length} campaign(s) from ${chosen}: ${files.join(", ")}`);

// The workloads, as the manifests declare them *today* — through the harness, which is
// the only thing in this repository that reads a `workload.yaml`. A YAML parser here
// would be a second reader of a file whose schema is generated from the Rust structs,
// and the two would drift the first time one of them was taught something.
//
// `--json` spells the checksum as a string, so this parse is safe. Diagnostics go to
// stderr; stdout carries the JSON alone.
function workloads() {
  const json = execFileSync(
    "cargo",
    ["run", "--quiet", "--", "workload", "list", "--json"],
    // `benchmarks/` is relative to the repository root, which is where the harness is
    // normally run from.
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );
  return JSON.parse(json);
}

let declared;
try {
  declared = workloads();
} catch (error) {
  fail(
    `cannot list the workloads: ${error.message}\n` +
      "The site describes the work from the manifests, and the harness is what reads them.\n" +
      "A Rust toolchain is required to build the site — the same one `npm run wasm` needs.",
  );
}

// What the *routes* are made of: one page per workload, one page per campaign, and a
// sidebar that lists both. Imported by Astro at build time — never fetched.
//
// A workload with no campaign is kept, deliberately: it is work somebody declared and
// nobody has measured yet, and the page says so. A campaign whose workload no longer
// exists on disk is kept too — it *ran*, and deleting the manifest afterwards does not
// unrun it.
mkdirSync(generated, { recursive: true });
const campaigns = files.map(header);
writeFileSync(
  join(generated, "site.json"),
  `${JSON.stringify({ workloads: declared, campaigns }, null, 2)}\n`,
);
console.log(
  `published ${declared.length} workload(s): ${declared.map((workload) => workload.id).join(", ")}`,
);
