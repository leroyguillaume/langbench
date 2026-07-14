//! Floating-point semantics the kernels are compiled under.
//!
//! A domain type, not a CLI type: it names a column of every sample, and it is
//! read back by anything that reads a campaign — including the WebAssembly
//! build, which has no `clap` in it. Only the `ValueEnum` derive belongs to the
//! command line, and it is gated on the `cli` feature.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Floating-point semantics the kernels are compiled under.
///
/// The axis is FP semantics, not "optimization on/off": every mode is `-O3`.
/// See `site/src/content/methodology.md#floating-point-modes`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[schemars(rename_all = "lowercase")]
pub enum FpMode {
    /// `-ffp-contract=off`, no fast-math. Bit-reproducible IEEE 754.
    Strict,
    /// FMA contraction allowed: bit-different, but more accurate.
    Fma,
    /// `-ffast-math`: reassociation allowed, precision sold for speed.
    Fast,
}

impl FpMode {
    pub const ALL: [Self; 3] = [Self::Strict, Self::Fma, Self::Fast];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Fma => "fma",
            Self::Fast => "fast",
        }
    }

    /// Parse a mode as a `bench.yaml` spells it.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}

impl fmt::Display for FpMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_serialization() {
        for mode in FpMode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{mode}\""));
        }
    }

    #[test]
    fn parse_round_trips_every_mode_and_rejects_anything_else() {
        for mode in FpMode::ALL {
            assert_eq!(FpMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(FpMode::parse("ffast-math"), None);
    }
}
