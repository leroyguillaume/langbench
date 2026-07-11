//! The website's only entry point into the harness.
//!
//! The site fetches `samples.ndjson` **as text** and hands the bytes straight to
//! [`analyze`]. It never calls `JSON.parse` on a campaign, and that is not a
//! stylistic preference: `checksum` is a 64-bit integer, a JavaScript `Number` is
//! an IEEE 754 double, and every integer past 2^53 comes out of `JSON.parse`
//! quietly rounded. The correctness gate of this harness would be corrupted by
//! the act of displaying it. So the file is parsed here, by `serde_json`, in
//! Rust — and the checksums leave through [`crate::analysis`], which serializes
//! them as strings. See `METHODOLOGY.md#the-strict-mode-invariant`.
//!
//! The second reason this module exists at all is [`crate::analysis`]: the site
//! gets the harness's own min-of-N, its own bucketing, its own definition of
//! startup. A TypeScript re-implementation would have been a second definition
//! of what this project measures.

use serde::Deserialize;
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::analysis::{self, Options};
use crate::compare::{self, Selection};
use crate::sample;

/// Route a Rust panic to the browser console instead of an opaque `unreachable`.
///
/// Idempotent, and the site calls it once at startup.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// What the site is allowed to vary. Mirrors [`Options`]; separate only because
/// every field is optional here, and a missing one means "the default the report
/// itself would have used".
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct WebOptions {
    include_warmup: bool,
}

impl From<WebOptions> for Options {
    fn from(options: WebOptions) -> Self {
        Self {
            include_warmup: options.include_warmup,
        }
    }
}

/// Parse a campaign and summarize it: the whole API.
///
/// `ndjson` is the literal content of a `samples.ndjson`, header line included.
/// `options` is a `WebOptions` object, or `undefined` for the defaults.
///
/// Returns the [`analysis::Analysis`] as a plain JavaScript object. Timings are
/// numbers — the site sorts and plots them; checksums are strings — the site
/// displays them and never does arithmetic on them.
#[wasm_bindgen]
pub fn analyze(
    ndjson: &str,
    options: wasm_bindgen::JsValue,
) -> Result<wasm_bindgen::JsValue, JsError> {
    let options = web_options(options)?;
    let analysis = analysis::analyze(&recording(ndjson)?, options.into());
    into_js(&analysis, "the analysis")
}

/// Two rows of one campaign, head to head — and whether the gap between them is
/// larger than the noise the campaign carries.
///
/// `selection` is a [`Selection`]: the algorithm, and a `{backend, mode}` on each
/// side. The pair is named, never indexed — a row number is a property of the
/// sort somebody clicked, and it does not survive a reload.
///
/// The verdict is the harness's, not the browser's. What counts as a difference
/// is a definition of what this project measures, and it has exactly one home.
/// See [`crate::compare`].
///
/// Errs on a row this campaign never measured, rather than inventing a zero.
#[wasm_bindgen]
pub fn compare(
    ndjson: &str,
    options: wasm_bindgen::JsValue,
    selection: wasm_bindgen::JsValue,
) -> Result<wasm_bindgen::JsValue, JsError> {
    let options = web_options(options)?;
    let selection: Selection = serde_wasm_bindgen::from_value(selection)
        .map_err(|error| JsError::new(&format!("invalid selection: {error}")))?;

    let analysis = analysis::analyze(&recording(ndjson)?, options.into());
    let comparison = compare::compare(&analysis, &selection)
        .map_err(|error| JsError::new(&format!("{error:#}")))?;
    into_js(&comparison, "the comparison")
}

/// The campaign, parsed by `serde_json` — never by `JSON.parse`. See the header.
fn recording(ndjson: &str) -> Result<sample::Recording, JsError> {
    sample::parse(ndjson).map_err(|error| JsError::new(&format!("{error:#}")))
}

fn web_options(options: wasm_bindgen::JsValue) -> Result<WebOptions, JsError> {
    if options.is_undefined() || options.is_null() {
        return Ok(WebOptions::default());
    }
    serde_wasm_bindgen::from_value(options)
        .map_err(|error| JsError::new(&format!("invalid options: {error}")))
}

/// `serde_json` first, then into JS: `serde_wasm_bindgen` would refuse the
/// `serialize_with` that turns a `u64` checksum into a string, and a `u64` it
/// serialized itself would arrive as a `BigInt` that no chart library accepts.
/// `JSON.parse`, on a string this module produced. Safe where a `JSON.parse` on
/// the raw campaign would not be: every wide integer left as a string.
fn into_js<T: serde::Serialize>(value: &T, what: &str) -> Result<wasm_bindgen::JsValue, JsError> {
    let json = serde_json::to_string(value)
        .map_err(|error| JsError::new(&format!("serializing {what}: {error}")))?;
    js_sys::JSON::parse(&json).map_err(|_| JsError::new(&format!("{what} is not valid JSON")))
}
