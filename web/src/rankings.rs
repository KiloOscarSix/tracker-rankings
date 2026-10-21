use std::cmp::Ordering;
use std::collections::HashMap;

use rankings_core::axis::Axis;
use rankings_core::category::Category;
use rankings_core::config::ScoringConfig;
use rankings_core::score::AxisScore;
use rankings_core::tier::{Tier, assign_tiers};

use crate::queries::trackers::{ListedTracker, ScoreRow};

#[derive(Debug, Clone)]
pub struct RankingRow {
    pub tracker: ListedTracker,
    pub tier: Option<Tier>,
    /// Indexed by position in [`Axis::ALL`].
    pub scores: [Option<AxisScore>; 5],
}

#[must_use]
pub const fn axis_index(axis: Axis) -> usize {
    match axis {
        Axis::Overall => 0,
        Axis::Content => 1,
        Axis::Community => 2,
        Axis::Retention => 3,
        Axis::Longevity => 4,
    }
}

#[must_use]
pub fn search_haystack(tracker: &ListedTracker) -> String {
    format!("{} {} {}", tracker.name, tracker.abbr, tracker.slug).to_lowercase()
}

#[must_use]
pub fn matches_search(tracker: &ListedTracker, needle: &str) -> bool {
    search_haystack(tracker).contains(&needle.trim().to_lowercase())
}

/// Tiers are assigned across every listed tracker before the category filter
/// applies, so a tracker keeps its tier whichever filter is active.
#[must_use]
pub fn build(
    trackers: Vec<ListedTracker>,
    scores: Vec<ScoreRow>,
    filter: Option<Category>,
    sort: Axis,
    cfg: &ScoringConfig,
) -> Vec<RankingRow> {
    let mut rows: Vec<RankingRow> = trackers
        .into_iter()
        .map(|tracker| RankingRow { tracker, tier: None, scores: [None; 5] })
        .collect();

    let positions: HashMap<i32, usize> =
        rows.iter().enumerate().map(|(i, row)| (row.tracker.id, i)).collect();
    for score in scores {
        let slot = positions
            .get(&score.tracker_id)
            .and_then(|i| rows.get_mut(*i))
            .and_then(|row| row.scores.get_mut(axis_index(score.axis)));
        if let Some(slot) = slot {
            *slot = Some(score.score);
        }
    }

    let scored: Vec<&mut RankingRow> = rows
        .iter_mut()
        .filter(|row| score_of(row, Axis::Overall).is_some())
        .collect();
    let overall: Vec<AxisScore> =
        scored.iter().filter_map(|row| score_of(row, Axis::Overall)).collect();
    for (row, tier) in scored.into_iter().zip(assign_tiers(&overall, cfg)) {
        row.tier = tier;
    }

    rows.retain(|row| {
        filter.is_none_or(|category| row.tracker.categories.contains(&category))
    });

    rows.sort_by(|a, b| {
        let by_score = match (score_of(a, sort), score_of(b, sort)) {
            (Some(x), Some(y)) => y.score.total_cmp(&x.score),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        by_score.then_with(|| a.tracker.name.cmp(&b.tracker.name))
    });

    rows
}

fn score_of(row: &RankingRow, axis: Axis) -> Option<AxisScore> {
    row.scores.get(axis_index(axis)).copied().flatten()
}
