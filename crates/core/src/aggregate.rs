use std::collections::BTreeMap;

use crate::axis::Axis;
use crate::config::ScoringConfig;
use crate::score::{AxisScore, Rating, Tally, bayesian, global_mean, tally};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisRating {
    pub tracker_id: i32,
    pub axis: Axis,
    pub rating: Rating,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerAxisScore {
    pub tracker_id: i32,
    pub axis: Axis,
    pub score: AxisScore,
}

/// Bayesian score of every rated (tracker, axis) pair, with `m` the weighted mean
/// of all ratings on that axis. Sorted by tracker id, then axis.
#[must_use]
pub fn aggregate(
    ratings: &[AxisRating],
    cfg: &ScoringConfig,
) -> Vec<TrackerAxisScore> {
    let mut groups: BTreeMap<(i32, Axis), Vec<Rating>> = BTreeMap::new();
    for r in ratings {
        groups.entry((r.tracker_id, r.axis)).or_default().push(r.rating);
    }

    let tallies: Vec<((i32, Axis), Tally)> = groups
        .iter()
        .map(|(key, group)| (*key, tally(group, &cfg.weights)))
        .collect();

    let means: BTreeMap<Axis, f64> = Axis::ALL
        .into_iter()
        .map(|axis| {
            let on_axis: Vec<Tally> = tallies
                .iter()
                .filter(|((_, a), _)| *a == axis)
                .map(|(_, t)| *t)
                .collect();
            (axis, global_mean(&on_axis, cfg.fallback_mean))
        })
        .collect();

    tallies
        .iter()
        .map(|((tracker_id, axis), t)| {
            let mean = means.get(axis).copied().unwrap_or(cfg.fallback_mean);
            TrackerAxisScore {
                tracker_id: *tracker_id,
                axis: *axis,
                score: bayesian(t, mean, cfg.confidence),
            }
        })
        .collect()
}
