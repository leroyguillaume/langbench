//! Energy, from the CPU package counters the kernel exposes under `powercap`.
//!
//! This is the one measurement the container cannot take for itself. `cpu.stat`
//! and `memory.peak` are cgroup files: they are namespaced, so the entrypoint
//! reads its own and reports it. RAPL is not. `/sys/class/powercap` describes the
//! *socket*, not a cgroup, and it is invisible from inside a container. So the
//! harness reads it, from the host, around the `docker run` — which means the
//! number this module produces is an **envelope**, and it is honest about it:
//!
//! - It is the energy the whole package burned while the container ran, and that
//!   includes `docker`'s own client, the daemon's work creating and reaping the
//!   container, and whatever else was awake on the machine.
//! - It is therefore the energy equivalent of `wall_ns`, never of `elapsed_ns`.
//!   There is no internal counterpart, and there cannot be one: the kernel has no
//!   per-cgroup joule.
//!
//! That is the same trade the wall-clock already makes, and the same answer:
//! publish the envelope, say what is in it, and let the reader subtract nothing.
//! See `METHODOLOGY.md#energy-is-an-envelope`.
//!
//! Two ways this returns nothing, and both are facts rather than bugs:
//!
//! - The counters do not exist. RAPL is x86 (Intel and AMD both drive the same
//!   `intel-rapl` powercap zones); an AArch64 host has no equivalent.
//! - They exist and are unreadable. Since the PLATYPUS side-channel, distributions
//!   ship `energy_uj` as root-only.
//!
//! In both cases the samples carry `energy_uj: null` and the machine record
//! carries a warning that says why. An absence is a published fact.

use std::fs;
use std::path::{Path, PathBuf};

use tracing::debug;

/// Where the kernel publishes the CPU package energy counters.
const POWERCAP: &str = "/sys/class/powercap";

/// One RAPL package domain: a CPU socket.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Domain {
    /// The monotonic microjoule counter. It wraps.
    energy: PathBuf,
    /// The value it wraps at, read once — it is a property of the hardware.
    max_range: u64,
}

/// The host's package counters, discovered once and read around every run.
///
/// Empty when the machine has none, or hides them behind root. `read` then
/// returns `None`, and every sample of the campaign says so.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnergyMeter {
    domains: Vec<Domain>,
}

/// The counters, at one instant. Positionally aligned with the meter's domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reading(Vec<u64>);

impl EnergyMeter {
    /// Find the package domains this host exposes *and* lets us read.
    ///
    /// Only the top-level `intel-rapl:<n>` zones. The nested ones —
    /// `intel-rapl:0:0` is the `core` sub-domain of package 0 — are *subsets* of
    /// their parent, and summing a package with its own children would count the
    /// same joules twice.
    pub fn detect() -> Self {
        let mut domains = Vec::new();
        let Ok(entries) = fs::read_dir(POWERCAP) else {
            debug!("no {POWERCAP}: this host publishes no energy counters");
            return Self::default();
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !is_package_zone(name) {
                continue;
            }
            let path = entry.path();
            let energy = path.join("energy_uj");
            // Probed by *reading* it, never by stat: the file exists on a host that
            // refuses to let us open it, and a meter that discovers that only once
            // the campaign is running would report the first sample and no other.
            let (Some(_), Some(max_range)) = (
                read_u64(&energy),
                read_u64(&path.join("max_energy_range_uj")),
            ) else {
                debug!(
                    zone = name,
                    "energy counter present but unreadable; needs root"
                );
                continue;
            };
            domains.push(Domain { energy, max_range });
        }

        // Sorted so a `Reading` taken before a run lines up with the one taken
        // after it. `read_dir` promises no order.
        domains.sort_by(|left, right| left.energy.cmp(&right.energy));
        debug!(packages = domains.len(), "energy counters");
        Self { domains }
    }

    /// Whether this campaign will be able to report energy at all.
    pub fn available(&self) -> bool {
        !self.domains.is_empty()
    }

    /// How the counters were read, for the machine record. `None` when they
    /// could not be.
    pub fn source(&self) -> Option<String> {
        self.available().then(|| {
            format!(
                "powercap RAPL ({} package domain{})",
                self.domains.len(),
                if self.domains.len() == 1 { "" } else { "s" },
            )
        })
    }

    /// Every package counter, now. `None` if any of them has stopped answering —
    /// a partial sum is not a smaller measurement, it is a wrong one.
    pub fn read(&self) -> Option<Reading> {
        if self.domains.is_empty() {
            return None;
        }
        let mut values = Vec::with_capacity(self.domains.len());
        for domain in &self.domains {
            values.push(read_u64(&domain.energy)?);
        }
        Some(Reading(values))
    }

    /// The microjoules burned between two readings, summed over every package.
    ///
    /// The counter is monotonic *until it wraps*, which it does — at
    /// `max_energy_range_uj`, roughly once an hour on a busy socket. A naive
    /// subtraction would produce a colossal negative number, and on `u64` a
    /// colossal positive one. A run that appears to have used more energy than the
    /// machine can produce is not a measurement; the wrap is corrected here.
    ///
    /// `None` if either reading is absent, or if it came from a different meter.
    pub fn delta(&self, before: Option<&Reading>, after: Option<&Reading>) -> Option<u64> {
        let (before, after) = (before?, after?);
        if before.0.len() != self.domains.len() || after.0.len() != self.domains.len() {
            return None;
        }
        let mut total: u64 = 0;
        for (index, domain) in self.domains.iter().enumerate() {
            let (start, end) = (before.0[index], after.0[index]);
            let burned = if end >= start {
                end - start
            } else {
                // One wrap, and only one: a container that ran long enough to wrap
                // the counter twice would need hours, and `--run-timeout` caps it
                // far below that.
                domain.max_range.saturating_sub(start).saturating_add(end)
            };
            total = total.saturating_add(burned);
        }
        Some(total)
    }
}

/// `intel-rapl:0` is a package. `intel-rapl:0:0` is one of its sub-domains, and
/// adding it to its own parent would count those joules twice.
///
/// AMD is not a special case: Zen exposes its RAPL MSRs through the very same
/// `intel-rapl` zones, misleading name and all.
fn is_package_zone(name: &str) -> bool {
    name.strip_prefix("intel-rapl:")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meter(max_ranges: &[u64]) -> EnergyMeter {
        EnergyMeter {
            domains: max_ranges
                .iter()
                .enumerate()
                .map(|(index, &max_range)| Domain {
                    energy: PathBuf::from(format!("/sys/class/powercap/intel-rapl:{index}")),
                    max_range,
                })
                .collect(),
        }
    }

    #[test]
    fn a_package_zone_is_the_top_level_one_never_its_sub_domains() {
        assert!(is_package_zone("intel-rapl:0"));
        assert!(is_package_zone("intel-rapl:12"));
        // The `core` and `uncore` sub-domains: subsets of package 0, and summing
        // them with it would double-count every joule they measure.
        assert!(!is_package_zone("intel-rapl:0:0"));
        assert!(!is_package_zone("intel-rapl:1:2"));
        assert!(!is_package_zone("intel-rapl"));
        assert!(!is_package_zone("dtpm"));
    }

    #[test]
    fn energy_is_summed_across_every_package() {
        let meter = meter(&[1_000_000, 1_000_000]);
        let before = Reading(vec![100, 200]);
        let after = Reading(vec![400, 900]);
        assert_eq!(meter.delta(Some(&before), Some(&after)), Some(300 + 700));
    }

    /// The counter wraps at `max_energy_range_uj`, and a plain subtraction would
    /// report a run that burned more energy than the machine can produce.
    #[test]
    fn a_counter_that_wrapped_reports_the_energy_it_actually_burned() {
        let meter = meter(&[1_000]);
        let before = Reading(vec![900]);
        let after = Reading(vec![100]);
        // 100 µJ to the ceiling, then 100 µJ past zero.
        assert_eq!(meter.delta(Some(&before), Some(&after)), Some(200));
    }

    #[test]
    fn a_meter_with_no_counter_measures_nothing_rather_than_zero() {
        let meter = EnergyMeter::default();
        assert!(!meter.available());
        assert_eq!(meter.read(), None);
        assert_eq!(meter.source(), None);
        assert_eq!(meter.delta(None, None), None);
    }

    /// Half a reading is not half a measurement. If the counters stopped answering
    /// mid-run, the sample has no energy — never a partial sum, which would look
    /// like a frugal backend.
    #[test]
    fn a_reading_that_does_not_match_the_meter_is_refused() {
        let meter = meter(&[1_000, 1_000]);
        let partial = Reading(vec![10]);
        let full = Reading(vec![10, 20]);
        assert_eq!(meter.delta(Some(&partial), Some(&full)), None);
        assert_eq!(meter.delta(Some(&full), None), None);
    }

    #[test]
    fn the_source_names_how_many_packages_it_summed() {
        assert_eq!(
            meter(&[1_000]).source().unwrap(),
            "powercap RAPL (1 package domain)",
        );
        assert_eq!(
            meter(&[1_000, 1_000]).source().unwrap(),
            "powercap RAPL (2 package domains)",
        );
    }
}
