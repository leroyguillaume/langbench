//! The harness, as a library.
//!
//! `langbench` is a binary, and the binary is what measures. This library exists
//! for one reason: the website has to read a campaign back, and it must do so
//! with *this* code — the same bucketing, the same min-of-N, the same
//! definition of startup — rather than with a TypeScript re-implementation that
//! would be a second, silently diverging definition of what this project
//! measures. See `crate::analysis`.
//!
//! Two feature flags carve the crate accordingly:
//!
//! - `cli` (default): everything that touches the machine — discovery, Docker,
//!   the campaign, the Markdown renderer. The `langbench` binary needs it.
//! - `wasm`: the `wasm-bindgen` boundary the website calls.
//!
//! Between them sits the part that is nothing but data and arithmetic —
//! [`mode`], [`sample`], [`stats`], [`analysis`], [`workload`], the [`machine`]
//! types — which compiles to `wasm32-unknown-unknown` with no `std::process` and
//! no Docker anywhere in sight.

pub mod analysis;
pub mod compare;
pub mod machine;
pub mod mode;
pub mod sample;
pub mod stats;
pub mod workload;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod discovery;
#[cfg(feature = "cli")]
pub mod engine;
#[cfg(feature = "cli")]
pub mod output;
#[cfg(feature = "cli")]
pub mod report;
#[cfg(feature = "cli")]
pub mod runner;
#[cfg(feature = "cli")]
pub mod shutdown;

#[cfg(feature = "wasm")]
pub mod wasm;
