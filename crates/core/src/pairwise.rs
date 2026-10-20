use std::collections::BTreeMap;

use crate::bradley_terry::{Matchup, fit};
use crate::config::ScoringConfig;
use crate::error::PairwiseError;
use crate::weight::Voter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairwiseVote {
    pub winner_id: i32,
    pub loser_id: i32,
    pub voter: Voter,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerStrength {
    pub tracker_id: i32,
    pub strength: f64,
    pub vote_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairwiseRanking {
    pub strengths: Vec<TrackerStrength>,
    pub iterations: u32,
    pub converged: bool,
}

/// Bradley–Terry strengths of every tracker with a pairwise vote
pub fn rank(
    votes: &[PairwiseVote],
    cfg: &ScoringConfig,
) -> Result<PairwiseRanking, PairwiseError> {
    let mut index: BTreeMap<i32, usize> = BTreeMap::new();
    let mut counts: Vec<u32> = Vec::new();
    let mut matchups = Vec::with_capacity(votes.len());

    for vote in votes {
        let mut dense = [0; 2];
        for (slot, id) in dense.iter_mut().zip([vote.winner_id, vote.loser_id]) {
            let next = index.len();
            let i = *index.entry(id).or_insert(next);
            if i == counts.len() {
                counts.push(0);
            }
            if let Some(count) = counts.get_mut(i) {
                *count = count.saturating_add(1);
            }
            *slot = i;
        }
        let [winner, loser] = dense;
        matchups.push(Matchup {
            winner,
            loser,
            weight: cfg.weights.weight(vote.voter),
        });
    }

    let fit = fit(index.len(), &matchups, &cfg.bradley_terry)?;
    let strengths = index
        .iter()
        .filter_map(|(&tracker_id, &i)| {
            Some(TrackerStrength {
                tracker_id,
                strength: fit.strengths.get(i).copied()?,
                vote_count: counts.get(i).copied()?,
            })
        })
        .collect();

    Ok(PairwiseRanking {
        strengths,
        iterations: fit.iterations,
        converged: fit.converged,
    })
}
