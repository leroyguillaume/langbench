//! What a workload is: the work itself, declared once, beside the
//! implementations that race to do it.
//!
//! A workload is **not** an algorithm. Mandelbrot is one; a JSON parser, an HTTP
//! server, a cold start are others. What they have in common is the only thing
//! this file asserts: something has to be done, it is sized by a few parameters,
//! and — when the work is deterministic — every honest implementation of it
//! agrees on one answer.
//!
//! This module is data, not machinery: no filesystem, no Docker, nothing to spawn.
//! It compiles to `wasm32-unknown-unknown` along with [`crate::sample`], because
//! the campaign header carries a **snapshot** of the workload it ran, and the site
//! reads that header. The same struct is therefore both what `workload.yaml`
//! declares and what a campaign records — one definition, so the two cannot drift.
//! See `METHODOLOGY.md#repository-layout`.

use std::fmt;

use anyhow::{Context, Result, bail, ensure};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Deserializer, Serialize};

/// The file that declares a workload. Its presence is what makes a directory the
/// root of one.
pub const MANIFEST: &str = "workload.yaml";

/// What `workload.yaml` declares — and, verbatim, what a campaign snapshots into
/// its header.
///
/// `deny_unknown_fields`, like the implementation manifest: a misspelled key must
/// fail the campaign rather than be quietly ignored. The numbers would still come
/// out; they would just be wrong about what produced them.
///
/// **`kebab-case` coming in, `snake_case` going out**, and the two directions are
/// not the same audience. A manifest is typed by a person, and YAML is written in
/// kebab wherever people write it. Everything downstream is read by a machine —
/// `samples.ndjson`, the CSV, the browser — and that wire speaks `snake_case` end to
/// end, so that `jq '.elapsed_ns'` works and no consumer needs a translation table.
/// This struct sits on the boundary because it is *both*: the file you write and the
/// snapshot the campaign records. So it reads one and writes the other, and only one
/// key is even affected today.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all(deserialize = "kebab-case"))]
#[schemars(title = "langbench workload manifest", rename_all = "kebab-case")]
pub struct Workload {
    /// What this workload is called. Declared, never parsed out of the directory
    /// name — the path locates the file, the file says what it is.
    pub id: String,
    /// What the work *is*, in one paragraph. It is published beside the results,
    /// because a timing without the work it timed is a number about nothing.
    pub description: String,
    /// The directories holding this workload's implementations, each relative to
    /// this manifest.
    ///
    /// Declared, not walked for. The harness could perfectly well recurse from here
    /// and take every `bench.yaml` it found, and that would mean the *position* of a
    /// directory decides whether it is measured — which is the path being metadata
    /// again, the thing this project refuses everywhere else. `source:` is declared
    /// for the same reason: the manifest says what is in, and nothing is in by
    /// accident.
    ///
    /// A directory that holds no `bench.yaml` is warned about and skipped: it is an
    /// implementation somebody started, or moved, and a campaign that failed on it
    /// would be a campaign held hostage by an empty folder.
    pub implementations: Vec<String>,
    /// How the work is sized, and what the kernels are handed on the command line.
    ///
    /// A list and not a mapping, because **the order is the `argv` order** and a
    /// list is the only YAML shape that is ordered by construction. A mapping would
    /// leave the kernels' arguments at the mercy of whatever sorted its keys.
    pub params: Vec<Param>,
    /// The value every `strict` run of this workload must produce, for *these*
    /// params.
    ///
    /// Optional, and the distinction matters. Without it, a campaign can only check
    /// that its backends agree *with each other* — which a campaign where every
    /// backend is wrong the same way passes with flying colours, and which makes no
    /// claim at all across two campaigns. With it, the answer is pinned to a value
    /// that outlives any single run.
    ///
    /// It is a property of `(workload, params)`, never of the workload alone:
    /// override a param and this no longer applies to what ran. The campaign says so
    /// rather than checking against a number that describes different work.
    ///
    /// **A string on every wire, an integer in the manifest.** It is 64 bits wide and
    /// a JavaScript number is a double, so a checksum that crossed into a browser as
    /// a number would silently lose its low bits past 2^53 — and this one is the
    /// correctness gate of the whole project. `workload.yaml` is written by a human,
    /// so it takes a plain integer; everything downstream of it is read by a machine
    /// with a 53-bit mantissa, so it gets a string. The harness never does arithmetic
    /// on it: it compares it, and it prints it.
    ///
    /// The manifest spells it `strict-checksum`; the campaign header it was written
    /// into spells it `strict_checksum`. The alias is what lets one struct read both
    /// — and it is not a courtesy to old files: this struct *round-trips*, so without
    /// it `langbench report` could not read back the header the campaign it just ran
    /// wrote.
    #[serde(
        default,
        alias = "strict_checksum",
        deserialize_with = "checksum",
        serialize_with = "crate::analysis::as_string"
    )]
    #[schemars(with = "Option<u64>")]
    pub strict_checksum: Option<u64>,
}

/// A checksum as either the manifest or the wire spells it: an integer, or a string
/// of digits.
///
/// Both, and deliberately: a manifest is typed by a person and a bare `1038538536`
/// is what they will write; a campaign header is read by a browser, where the same
/// value has to arrive as a string or arrive rounded.
fn checksum<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Integer(u64),
        Text(String),
    }

    match Option::<Either>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Either::Integer(value)) => Ok(Some(value)),
        Some(Either::Text(text)) => text.trim().parse().map(Some).map_err(|_| {
            serde::de::Error::custom(format!(
                "`{text}` is not a checksum: it is a 64-bit integer, written as digits",
            ))
        }),
    }
}

/// One knob of a workload, and the value this campaign ran it at.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all(deserialize = "kebab-case"))]
#[schemars(rename_all = "kebab-case")]
pub struct Param {
    pub name: String,
    pub value: ParamValue,
}

/// A param's value: whatever scalar the manifest wrote.
///
/// The harness does no arithmetic on it. It reaches a kernel as one `argv` token
/// and reaches a report as one cell, so what it must do is survive the trip and
/// print as it was written — never `2048.0` for a grid of `2048`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// Tried first: `2048` is an integer, and a grid is not `2048.0` pixels wide.
    Integer(i64),
    Boolean(bool),
    Text(String),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => write!(f, "{value}"),
            Self::Boolean(value) => write!(f, "{value}"),
            Self::Text(value) => f.write_str(value),
        }
    }
}

impl ParamValue {
    /// Read a value as a command line spells it: `--param grid_size=256`.
    ///
    /// Everything on a command line arrives as text, so the type has to be
    /// recovered. An integer is one; anything else is what it looks like.
    fn parse(value: &str) -> Self {
        if let Ok(integer) = value.parse::<i64>() {
            return Self::Integer(integer);
        }
        match value {
            "true" => Self::Boolean(true),
            "false" => Self::Boolean(false),
            other => Self::Text(other.to_owned()),
        }
    }
}

impl Workload {
    /// Read a `workload.yaml`.
    pub fn parse(text: &str) -> Result<Self> {
        let workload: Self =
            serde_norway::from_str(text).context("parsing the workload manifest")?;
        ensure!(
            !workload.id.trim().is_empty(),
            "a workload declares an `id`: it is what names the campaign, and what \
             `langbench workload run` is given",
        );
        Ok(workload)
    }

    /// The parameters, in declaration order, as a kernel receives them.
    ///
    /// This is the whole contract between a workload and the implementations under
    /// it: the container is run as `run <params…> <threads>`, and a kernel reads
    /// them positionally. The thread count is not here — it is resolved by the
    /// harness from the machine, not declared by the work.
    pub fn argv(&self) -> Vec<String> {
        self.params
            .iter()
            .map(|param| param.value.to_string())
            .collect()
    }

    /// The same workload, with some params overridden from the command line.
    ///
    /// A key that the workload never declared is an error, not a no-op: a
    /// `--param grdi_size=256` that silently changed nothing would publish a
    /// campaign at the wrong size under the right name.
    ///
    /// **An override that actually changes a value drops `strict_checksum`.** The
    /// declared reference is the answer to the declared work; ask for different
    /// work and it is not the answer to it. The snapshot in the header then honestly
    /// carries no reference, and the campaign falls back to checking that its
    /// backends agree with each other.
    pub fn with_overrides(&self, overrides: &[(String, String)]) -> Result<Self> {
        let mut overridden = self.clone();
        for (name, value) in overrides {
            let param = overridden
                .params
                .iter_mut()
                .find(|param| &param.name == name)
                .with_context(|| {
                    format!(
                        "`{name}` is not a param of the `{}` workload; it declares {}",
                        self.id,
                        self.param_names(),
                    )
                })?;
            param.value = ParamValue::parse(value);
        }

        if overridden.params != self.params {
            overridden.strict_checksum = None;
        }
        Ok(overridden)
    }

    fn param_names(&self) -> String {
        if self.params.is_empty() {
            return "none".to_owned();
        }
        self.params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
impl Workload {
    /// The workload the tests elsewhere in the crate measure.
    ///
    /// It declares **no** `strict_checksum`: the campaign fixtures check that the
    /// harness establishes a reference from the runs themselves, which is what a
    /// workload without a declared answer gets. The tests that are about the
    /// declared reference say so by setting it.
    pub fn fixture() -> Self {
        Self {
            id: "mandelbrot".to_owned(),
            description: "Escape-time Mandelbrot over a fixed grid.".to_owned(),
            implementations: Vec::new(),
            params: vec![
                Param {
                    name: "grid_size".to_owned(),
                    value: ParamValue::Integer(4096),
                },
                Param {
                    name: "max_iter".to_owned(),
                    value: ParamValue::Integer(1000),
                },
            ],
            strict_checksum: None,
        }
    }
}

/// Parse the `--param name=value` pairs a command line carries.
pub fn overrides(raw: &[String]) -> Result<Vec<(String, String)>> {
    raw.iter()
        .map(|pair| match pair.split_once('=') {
            Some((name, value)) if !name.trim().is_empty() => {
                Ok((name.trim().to_owned(), value.to_owned()))
            }
            _ => bail!("`{pair}` is not a param: write it as `name=value`, e.g. `grid_size=256`"),
        })
        .collect()
}

/// The workload manifest's JSON Schema, pretty-printed.
///
/// Generated from the struct the harness deserializes, never hand-written, for the
/// reason `bench.schema.json` is: a schema maintained by hand is a second
/// declaration of the format, and it drifts.
pub fn schema() -> Result<String> {
    serde_json::to_string_pretty(&schema_for!(Workload)).context("serializing the workload schema")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANDELBROT: &str = "id: mandelbrot\n\
                              description: Escape-time Mandelbrot over a fixed grid.\n\
                              implementations:\n\
                              \x20 - c-gcc\n\
                              params:\n\
                              \x20 - name: grid_size\n\
                              \x20   value: 2048\n\
                              \x20 - name: max_iter\n\
                              \x20   value: 1000\n\
                              strict-checksum: 1038538536\n";

    fn mandelbrot() -> Workload {
        Workload::parse(MANDELBROT).unwrap()
    }

    #[test]
    fn a_manifest_declares_the_work_and_how_it_is_sized() {
        let workload = mandelbrot();
        assert_eq!(workload.id, "mandelbrot");
        assert_eq!(workload.strict_checksum, Some(1_038_538_536));
        assert_eq!(
            workload.params,
            [
                Param {
                    name: "grid_size".to_owned(),
                    value: ParamValue::Integer(2048),
                },
                Param {
                    name: "max_iter".to_owned(),
                    value: ParamValue::Integer(1000),
                },
            ],
        );
    }

    /// The order of the list *is* the order of the kernel's arguments, and a kernel
    /// reads them positionally. This is the contract, and it is why `params` is a
    /// list and not a mapping.
    #[test]
    fn the_params_reach_the_kernel_in_declaration_order() {
        assert_eq!(mandelbrot().argv(), ["2048", "1000"]);
    }

    /// A grid of 2048 is 2048 pixels wide, not 2048.0 of them. The value prints as
    /// it was written, all the way to the kernel's `argv`.
    #[test]
    fn an_integer_param_does_not_become_a_float_on_the_way_to_the_kernel() {
        assert_eq!(ParamValue::Integer(2048).to_string(), "2048");
        assert_eq!(ParamValue::parse("2048"), ParamValue::Integer(2048));
        assert_eq!(
            ParamValue::parse("balanced"),
            ParamValue::Text("balanced".to_owned()),
        );
    }

    /// The declared reference answers the declared work. Ask for a smaller grid and
    /// it is simply not the answer to what ran — so the snapshot carries none, and
    /// says so, rather than checking against a number that describes other work.
    #[test]
    fn overriding_a_param_drops_the_reference_checksum() {
        let overridden = mandelbrot()
            .with_overrides(&[("grid_size".to_owned(), "256".to_owned())])
            .unwrap();

        assert_eq!(overridden.argv(), ["256", "1000"]);
        assert_eq!(overridden.strict_checksum, None);
    }

    /// Re-stating a param at the value it already has is not a change, and must not
    /// cost the campaign its reference: `--param grid_size=2048` on a workload that
    /// declares 2048 ran exactly the declared work.
    #[test]
    fn an_override_that_changes_nothing_keeps_the_reference_checksum() {
        let overridden = mandelbrot()
            .with_overrides(&[("grid_size".to_owned(), "2048".to_owned())])
            .unwrap();

        assert_eq!(overridden.strict_checksum, Some(1_038_538_536));
    }

    /// A typo in a param name must fail the campaign. Ignored, it would publish a
    /// campaign at the declared size under a name that promised another one.
    #[test]
    fn an_unknown_param_fails_the_campaign() {
        let error = mandelbrot()
            .with_overrides(&[("grdi_size".to_owned(), "256".to_owned())])
            .unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("grdi_size"), "{error}");
        assert!(error.contains("grid_size, max_iter"), "{error}");
    }

    #[test]
    fn a_param_is_written_name_equals_value() {
        assert_eq!(
            overrides(&["grid_size=256".to_owned()]).unwrap(),
            [("grid_size".to_owned(), "256".to_owned())],
        );
        assert!(overrides(&["grid_size".to_owned()]).is_err());
        assert!(overrides(&["=256".to_owned()]).is_err());
    }

    #[test]
    fn an_unknown_key_fails_the_manifest() {
        let error = Workload::parse(
            "id: mandelbrot\n\
             description: Fine.\n\
             params: []\n\
             strict-cheksum: 1\n",
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("strict-cheksum"));
    }

    /// The schema is generated from the struct the harness deserializes. If this
    /// ever needs updating by hand, the generator has been bypassed.
    #[test]
    fn the_schema_describes_the_manifest_the_harness_parses() {
        let schema = schema().unwrap();
        // The schema describes the file a person *writes*, so it spells the keys the
        // way the file does: kebab.
        for key in ["id", "description", "params", "strict-checksum"] {
            assert!(schema.contains(&format!("\"{key}\"")), "{key} is missing");
        }
        assert!(schema.contains("\"additionalProperties\": false"));
    }

    /// One struct, two audiences: kebab from the file a person writes, snake onto the
    /// wire a machine reads. And it round-trips — the campaign header this very struct
    /// serialized has to deserialize back, which is what the alias is for.
    #[test]
    fn the_manifest_reads_kebab_and_the_wire_writes_snake() {
        let workload = mandelbrot();
        let json = serde_json::to_string(&workload).unwrap();
        assert!(json.contains("\"strict_checksum\""), "{json}");
        assert!(!json.contains("strict-checksum"), "{json}");

        let round_tripped: Workload = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, workload);
    }

    /// The old spelling is not accepted in a manifest by accident: it is the one the
    /// header carries, and the alias is what lets the header be read back.
    #[test]
    fn a_manifest_that_spells_it_the_wire_way_is_still_read() {
        let workload = Workload::parse(&MANDELBROT.replace("strict-checksum", "strict_checksum"));
        assert_eq!(workload.unwrap().strict_checksum, Some(1_038_538_536));
    }
}
