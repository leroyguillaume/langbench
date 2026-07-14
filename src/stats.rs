//! Min-of-N, median and median absolute deviation.
//!
//! Contention noise is one-sided: it can only slow a run down, never speed it
//! up. So the minimum estimates the machine's capability, and the dispersion is
//! a verdict on the campaign rather than an error bar on the result.
//! See `site/src/content/methodology.md#sampling-and-what-may-be-concluded`.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Summary {
    pub n: usize,
    pub min: u64,
    pub median: u64,
    /// Median absolute deviation, in the same unit as the samples.
    pub mad: u64,
    /// MAD as a percentage of the median. Above ~2%, percentage-level claims
    /// are not defensible.
    pub mad_pct: f64,
}

/// `None` for an empty slice: a summary of nothing is not zero.
pub fn summarize(values: &[u64]) -> Option<Summary> {
    if values.is_empty() {
        return None;
    }
    let center = median(values);
    let deviations: Vec<u64> = values.iter().map(|value| value.abs_diff(center)).collect();
    let mad = median(&deviations);
    Some(Summary {
        n: values.len(),
        min: *values.iter().min().expect("values is not empty"),
        median: center,
        mad,
        mad_pct: if center == 0 {
            0.0
        } else {
            mad as f64 / center as f64 * 100.0
        },
    })
}

/// Lower median on even counts: the samples are timings, and interpolating
/// between two observations would invent one that never happened.
fn median(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() - 1) / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_slice_has_no_summary() {
        assert_eq!(summarize(&[]), None);
    }

    #[test]
    fn a_single_sample_has_no_dispersion() {
        let summary = summarize(&[42]).unwrap();
        assert_eq!(summary.n, 1);
        assert_eq!(summary.min, 42);
        assert_eq!(summary.median, 42);
        assert_eq!(summary.mad, 0);
        assert_eq!(summary.mad_pct, 0.0);
    }

    #[test]
    fn the_median_does_not_interpolate() {
        assert_eq!(median(&[10, 20, 30, 40]), 20);
        assert_eq!(median(&[10, 20, 30]), 20);
    }

    #[test]
    fn a_single_spike_moves_neither_the_minimum_nor_the_dispersion() {
        // MAD is robust by design: one clobbered round does not condemn a
        // campaign, and the minimum ignores it too. Isolated spikes are found
        // in the raw samples, not in the summary.
        let clean = summarize(&[100, 101, 102, 103, 104]).unwrap();
        let spiked = summarize(&[100, 101, 102, 103, 9_000]).unwrap();
        assert_eq!(clean.min, spiked.min);
        assert_eq!(clean.mad, spiked.mad);
    }

    #[test]
    fn a_broadly_noisy_campaign_raises_the_dispersion() {
        let clean = summarize(&[100, 101, 102, 103, 104]).unwrap();
        let noisy = summarize(&[60, 85, 102, 130, 170]).unwrap();
        assert!(noisy.mad_pct > clean.mad_pct);
    }

    #[test]
    fn dispersion_is_reported_as_a_percentage_of_the_median() {
        let summary = summarize(&[90, 100, 110]).unwrap();
        assert_eq!(summary.median, 100);
        assert_eq!(summary.mad, 10);
        assert!((summary.mad_pct - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_zero_median_does_not_divide_by_zero() {
        let summary = summarize(&[0, 0, 0]).unwrap();
        assert_eq!(summary.mad_pct, 0.0);
    }
}
