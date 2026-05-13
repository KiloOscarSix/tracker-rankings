use std::collections::BTreeMap;

use crate::config::BradleyTerryConfig;
use crate::error::PairwiseError;

/// One weighted pairwise vote between items indexed `0..n_items`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matchup {
    pub winner: usize,
    pub loser: usize,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    pub strengths: Vec<f64>,
    pub iterations: u32,
    pub converged: bool,
}

/// Bradley–Terry strengths fitted with the MM iteration of Hunter (2004),
/// "MM algorithms for generalized Bradley–Terry models", Ann. Statist. 32(1).
pub fn fit(
    n_items: usize,
    matchups: &[Matchup],
    cfg: &BradleyTerryConfig,
) -> Result<Fit, PairwiseError> {
    for m in matchups {
        for index in [m.winner, m.loser] {
            if index >= n_items {
                return Err(PairwiseError::OutOfRange { index, len: n_items });
            }
        }
        if m.winner == m.loser {
            return Err(PairwiseError::SelfMatch(m.winner));
        }
        if !(m.weight.is_finite() && m.weight > 0.0) {
            return Err(PairwiseError::InvalidWeight(m.weight));
        }
    }

    let mut wins = vec![0.0; n_items];
    let mut games: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for m in matchups {
        if let Some(w) = wins.get_mut(m.winner) {
            *w += m.weight;
        }
        let key = (m.winner.min(m.loser), m.winner.max(m.loser));
        *games.entry(key).or_insert(0.0) += m.weight;
    }

    let a = cfg.prior_games;
    let mut strengths = vec![1.0; n_items];

    for iteration in 1..=cfg.max_iterations {
        let mut denom: Vec<f64> =
            strengths.iter().map(|p| 2.0 * a / (p + 1.0)).collect();
        for (&(i, j), &n_ij) in &games {
            let (Some(p_i), Some(p_j)) = (strengths.get(i), strengths.get(j)) else {
                continue;
            };
            let t = n_ij / (p_i + p_j);
            if let Some(d) = denom.get_mut(i) {
                *d += t;
            }
            if let Some(d) = denom.get_mut(j) {
                *d += t;
            }
        }

        let next: Vec<f64> =
            wins.iter().zip(&denom).map(|(w, d)| (w + a) / d).collect();
        let max_change = next
            .iter()
            .zip(&strengths)
            .map(|(new, old)| ((new - old) / old).abs())
            .fold(0.0, f64::max);
        strengths = next;

        if max_change < cfg.tolerance {
            return Ok(Fit { strengths, iterations: iteration, converged: true });
        }
    }

    Ok(Fit { strengths, iterations: cfg.max_iterations, converged: false })
}
