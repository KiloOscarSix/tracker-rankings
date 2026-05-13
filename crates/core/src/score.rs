use crate::config::VoteWeights;
use crate::error::InvalidScore;
use crate::weight::Voter;

/// A 1–5 rating on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u8);

impl Score {
    pub const MAX: u8 = 5;
    pub const MIN: u8 = 1;

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<i16> for Score {
    type Error = InvalidScore;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        u8::try_from(value)
            .ok()
            .filter(|s| (Self::MIN..=Self::MAX).contains(s))
            .map(Self)
            .ok_or(InvalidScore(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rating {
    pub score: Score,
    pub voter: Voter,
}

/// Weighted totals of one tracker's ratings on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tally {
    pub weight: f64,
    pub weighted_sum: f64,
    pub count: u32,
}

/// One tracker's score on one axis; mirrors the `tracker_scores` columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisScore {
    pub score: f64,
    pub effective_weight: f64,
    pub vote_count: u32,
}

#[must_use]
pub fn tally(ratings: &[Rating], weights: &VoteWeights) -> Tally {
    ratings.iter().fold(Tally::default(), |mut acc, r| {
        let w = weights.weight(r.voter);
        acc.weight += w;
        acc.weighted_sum = w.mul_add(f64::from(r.score.get()), acc.weighted_sum);
        acc.count = acc.count.saturating_add(1);
        acc
    })
}

/// Weighted mean of every rating on an axis, or `fallback` when there are none.
#[must_use]
pub fn global_mean(tallies: &[Tally], fallback: f64) -> f64 {
    let total: f64 = tallies.iter().map(|t| t.weight).sum();
    if total > 0.0 {
        tallies.iter().map(|t| t.weighted_sum).sum::<f64>() / total
    } else {
        fallback
    }
}

/// Bayesian average `(C·m + Σw·s) / (C + Σw)`
#[must_use]
pub const fn bayesian(tally: &Tally, mean: f64, confidence: f64) -> AxisScore {
    let prior = confidence * mean;
    AxisScore {
        score: (prior + tally.weighted_sum) / (confidence + tally.weight),
        effective_weight: tally.weight,
        vote_count: tally.count,
    }
}
