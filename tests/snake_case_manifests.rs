//! Every key a manifest declares is `snake_case`.
//!
//! One vocabulary, from the YAML on disk to the NDJSON on the wire to the CSV to the
//! browser — no translation table, and nobody having to remember which of the four
//! spells a key which way. See `CLAUDE.md`.
//!
//! It is true today by accident: a Rust field is `snake_case` because the compiler
//! insists, and serde copies the name across verbatim. What this test guards is the
//! one move that would break it without a warning anywhere — a `#[serde(rename =
//! "strictChecksum")]` on a field, or a `rename_all` on a struct. The schemas are
//! generated from the very structs the harness deserializes, so they see exactly what
//! serde sees.

use langbench::{discovery, workload};
use serde_json::Value;

#[test]
fn every_key_of_both_manifests_is_snake_case() {
    let mut checked = 0usize;

    for (manifest, schema) in [
        ("bench.yaml", discovery::schema().unwrap()),
        ("workload.yaml", workload::schema().unwrap()),
    ] {
        let schema: Value = serde_json::from_str(&schema).unwrap();
        let keys = property_names(&schema);

        assert!(
            !keys.is_empty(),
            "{manifest}: the schema declares no property at all, so this test would pass \
             vacuously — the walk below has stopped finding them",
        );

        for key in &keys {
            assert!(
                is_snake_case(key),
                "{manifest} declares `{key}`, which is not snake_case. The wire speaks one \
                 vocabulary — the YAML, the NDJSON, the CSV and the browser all spell a key the \
                 same way — and a rename here is a translation table everywhere else.",
            );
        }
        checked += keys.len();
    }

    println!("{checked} manifest keys, all snake_case");
}

/// Every key under a `properties` object, at any depth — the manifest's own
/// vocabulary, and never JSON Schema's (`type`, `$ref`, `description`…), which is a
/// different language and not ours to name.
fn property_names(schema: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect(schema, &mut found);
    found.sort();
    found.dedup();
    found
}

fn collect(node: &Value, found: &mut Vec<String>) {
    match node {
        Value::Object(fields) => {
            if let Some(Value::Object(properties)) = fields.get("properties") {
                found.extend(properties.keys().cloned());
            }
            for value in fields.values() {
                collect(value, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect(item, found)),
        _ => {}
    }
}

/// `strict_checksum`, `grid_size`, `id`. Never `strictChecksum`, never `strict-checksum`.
fn is_snake_case(key: &str) -> bool {
    !key.is_empty()
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[test]
fn the_guard_would_catch_a_renamed_key() {
    assert!(is_snake_case("strict_checksum"));
    assert!(is_snake_case("id"));
    assert!(!is_snake_case("strictChecksum"));
    assert!(!is_snake_case("strict-checksum"));
    assert!(!is_snake_case("StrictChecksum"));
    assert!(!is_snake_case("_private"));
}
