//! Two audiences, two conventions, and neither may leak into the other.
//!
//! **A manifest is `kebab-case`.** `workload.yaml` and `bench.yaml` are typed by a
//! person, and that is how YAML is written wherever people write it.
//!
//! **The wire is `snake_case`.** `samples.ndjson`, the CSV, and the JSON that crosses
//! into the browser are read by machines, and they speak one vocabulary end to end —
//! so `jq '.elapsed_ns'` works, and no consumer needs a translation table. Kebab keys
//! would not even be reachable that way: `jq` would need `.["elapsed-ns"]`.
//!
//! [`langbench::workload::Workload`] straddles the boundary — it *is* the file you
//! write and the snapshot the campaign records — so it reads one and writes the
//! other. That is the one place this could silently rot, which is why it is tested
//! from both sides.

use langbench::mode::FpMode;
use langbench::sample::{Phase, Sample};
use langbench::workload::{Param, ParamValue, Workload};
use langbench::{discovery, workload};
use serde_json::Value;

#[test]
fn every_key_of_both_manifests_is_kebab_case() {
    for (manifest, schema) in [
        ("bench.yaml", discovery::schema().unwrap()),
        ("workload.yaml", workload::schema().unwrap()),
    ] {
        let keys = property_names(&serde_json::from_str(&schema).unwrap());
        assert!(
            !keys.is_empty(),
            "{manifest}: the schema declares no property at all, so this test would pass \
             vacuously — the walk below has stopped finding them",
        );

        for key in &keys {
            assert!(
                is_kebab_case(key),
                "{manifest} declares `{key}`, which is not kebab-case. A manifest is typed by a \
                 person, and these two are the only files in this project that are.",
            );
        }
    }
}

#[test]
fn every_key_on_the_wire_is_snake_case() {
    // The two records a campaign writes, and the struct that straddles the boundary.
    let wire = [
        (
            "the workload snapshot",
            serde_json::to_value(workload()).unwrap(),
        ),
        ("a sample", serde_json::to_value(sample()).unwrap()),
    ];

    for (record, value) in wire {
        let keys = object_keys(&value);
        assert!(!keys.is_empty(), "{record}: serialized to no key at all");

        for key in &keys {
            assert!(
                is_snake_case(key),
                "{record} puts `{key}` on the wire, which is not snake_case. The NDJSON, the CSV \
                 and the browser speak one vocabulary — and a kebab key is not even reachable \
                 with `jq '.{key}'`, which reads it as a subtraction.",
            );
        }
    }
}

/// [`Workload`] round-trips: it *wrote* the campaign header, and it has to read it
/// back — `langbench report` on the campaign `langbench workload run` just finished is
/// exactly that. It reads kebab and writes snake, so this is where the two
/// conventions could quietly stop meeting.
///
/// Today every key is a single word, spelled identically either way, and nothing
/// bridges anything. The day one is not, this test is what says so — rather than a
/// campaign that suddenly cannot be rendered.
#[test]
fn the_header_a_campaign_writes_is_a_header_it_can_read_back() {
    let workload = workload();
    let header = serde_json::to_string(&workload).unwrap();
    assert!(header.contains("\"checksum\""), "{header}");

    let read_back: Workload = serde_json::from_str(&header).unwrap();
    assert_eq!(read_back, workload);
}

fn workload() -> Workload {
    Workload {
        id: "mandelbrot".to_owned(),
        description: "Escape-time Mandelbrot over a fixed grid.".to_owned(),
        implementations: vec!["c-gcc".to_owned()],
        params: vec![Param {
            name: "grid_size".to_owned(),
            value: ParamValue::Integer(2048),
        }],
        checksum: Some(1_038_538_536),
    }
}

fn sample() -> Sample {
    Sample {
        workload: "mandelbrot".to_owned(),
        language: "c".to_owned(),
        compiler: Some("gcc".to_owned()),
        interpreter: None,
        description: "The reference C kernel.".to_owned(),
        comments: None,
        mode: FpMode::Strict,
        phase: Phase::Run,
        round: 1,
        warmup: false,
        cpu: 8,
        wall_ns: 313_600_000,
        elapsed_ns: 213_300_000,
        user_usec: 860_000,
        system_usec: 4_000,
        peak_bytes: Some(12_582_912),
        source_bytes: Some(2_048),
        checksum: Some(42),
        binary_bytes: None,
        binary_stripped_bytes: None,
        text_bytes: None,
    }
}

/// Every key under a `properties` object, at any depth — the manifest's own
/// vocabulary, and never JSON Schema's (`type`, `$ref`, `description`…), which is a
/// different language and not ours to name.
fn property_names(schema: &Value) -> Vec<String> {
    fn walk(node: &Value, found: &mut Vec<String>) {
        match node {
            Value::Object(fields) => {
                if let Some(Value::Object(properties)) = fields.get("properties") {
                    found.extend(properties.keys().cloned());
                }
                fields.values().for_each(|value| walk(value, found));
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, found)),
            _ => {}
        }
    }

    let mut found = Vec::new();
    walk(schema, &mut found);
    found.sort();
    found.dedup();
    found
}

/// Every key of a serialized record, at any depth.
fn object_keys(value: &Value) -> Vec<String> {
    fn walk(node: &Value, found: &mut Vec<String>) {
        match node {
            Value::Object(fields) => {
                found.extend(fields.keys().cloned());
                fields.values().for_each(|value| walk(value, found));
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, found)),
            _ => {}
        }
    }

    let mut found = Vec::new();
    walk(value, &mut found);
    found.sort();
    found.dedup();
    found
}

/// `strict-checksum`, `id`. Never `strict_checksum`, never `strictChecksum`.
fn is_kebab_case(key: &str) -> bool {
    !key.is_empty()
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `strict_checksum`, `elapsed_ns`, `id`. Never `strict-checksum`, never `elapsedNs`.
fn is_snake_case(key: &str) -> bool {
    !key.is_empty()
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[test]
fn the_guards_would_catch_a_key_from_the_other_side() {
    assert!(is_kebab_case("strict-checksum") && !is_snake_case("strict-checksum"));
    assert!(is_snake_case("elapsed_ns") && !is_kebab_case("elapsed_ns"));
    // A single word is both, which is why only one key in this project is even
    // affected — and why a guard that only checked the multi-word ones would pass
    // while spelling the next one wrong.
    assert!(is_kebab_case("id") && is_snake_case("id"));
    assert!(!is_kebab_case("strictChecksum") && !is_snake_case("strictChecksum"));
}
