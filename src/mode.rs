//! The ISA target the kernels are compiled for.
//!
//! A domain type, not a CLI type: it names a column of every sample, and it is
//! read back by anything that reads a campaign — including the WebAssembly
//! build, which has no `clap` in it. Only the `ValueEnum` derive belongs to the
//! command line, and it is gated on the `cli` feature.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The ISA target the kernels are compiled for.
///
/// The axis is *which instructions the code generator may emit* — not
/// floating-point semantics, and not "optimization on/off". Every mode is `-O3`,
/// and every mode is strict IEEE 754, so the checksum is the gate on both:
/// widening a vector reorders nothing.
///
/// The two values are the two answers a toolchain can give to "which machine is
/// this code for?", and *which answers a backend can give* is the subject of this
/// project. An ahead-of-time compiler has to choose, so it has both. A JIT
/// generates code on the machine it is running on and cannot do otherwise, so it
/// has only [`Native`] — not as a limitation, but as the thing it sells.
///
/// See `site/src/content/methodology.md#the-isa-target`.
///
/// [`Native`]: Mode::Native
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[schemars(rename_all = "lowercase")]
pub enum Mode {
    /// A pinned ISA baseline — `x86-64-v3`, `armv8.2-a` — identical for every
    /// backend on the architecture. The binary does not depend on the CPU that
    /// built it, which is what shipping to a fleet means.
    Baseline,
    /// Whatever this CPU offers, resolved by the toolchain against the machine it
    /// is on. Not reproducible across machines, and that is the point: it is what
    /// a JIT gets for free, and what an ahead-of-time compiler buys by giving up
    /// portability.
    Native,
}

impl Mode {
    pub const ALL: [Self; 2] = [Self::Baseline, Self::Native];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Native => "native",
        }
    }

    /// Parse a mode as a `bench.yaml` spells it.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_serialization() {
        for mode in Mode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{mode}\""));
        }
    }

    #[test]
    fn parse_round_trips_every_mode_and_rejects_anything_else() {
        for mode in Mode::ALL {
            assert_eq!(Mode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(Mode::parse("march=native"), None);
    }

    /// The floating-point modes this axis replaced, and they are aliases of
    /// nothing. `fma` and `fast` computed a *different number* — reading one back
    /// as a `baseline` sample would relabel a run that never happened, and a
    /// campaign carrying them predates the checksum being the gate on every row.
    /// They must fail to parse, loudly, rather than be quietly reinterpreted.
    #[test]
    fn the_floating_point_modes_are_gone_and_do_not_parse() {
        for legacy in ["strict", "fma", "fast"] {
            assert_eq!(Mode::parse(legacy), None);
        }
    }
}
