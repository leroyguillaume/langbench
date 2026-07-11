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
    let options: WebOptions = if options.is_undefined() || options.is_null() {
        WebOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|error| JsError::new(&format!("invalid options: {error}")))?
    };

    let recording = sample::parse(ndjson).map_err(|error| JsError::new(&format!("{error:#}")))?;
    let analysis = analysis::analyze(&recording, options.into());

    // `serde_json` first, then into JS: `serde_wasm_bindgen` would refuse the
    // `serialize_with` that turns a `u64` checksum into a string, and a `u64` it
    // serialized itself would arrive as a `BigInt` that no chart library accepts.
    let json = serde_json::to_string(&analysis)
        .map_err(|error| JsError::new(&format!("serializing the analysis: {error}")))?;
    js_sys_json_parse(&json)
}

/// `JSON.parse`, on a string this module produced. Safe where a `JSON.parse` on
/// the raw campaign would not be: every wide integer left as a string.
fn js_sys_json_parse(json: &str) -> Result<wasm_bindgen::JsValue, JsError> {
    js_sys::JSON::parse(json).map_err(|_| JsError::new("the analysis is not valid JSON"))
}
