//! Ties the crate's rebuild to the embedded report template.
//!
//! `src/report.rs` pulls `templates/report.md.liquid` in with `include_str!`,
//! so the template is part of the binary but lives outside `src/`. Declaring it
//! here makes Cargo rebuild when it changes — and, because a build script that
//! emits no `rerun-if-changed` is re-run on *any* file change in the package,
//! keeps the trigger set to exactly what matters.

const TEMPLATE: &str = "templates/report.md.liquid";

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed={TEMPLATE}");
}
