use crate::config::ScoringConfig;
use crate::score::AxisScore;

// Keeps a top fraction that lands exactly on a cumulative share in the lower tier
// despite float summation error.
const BOUNDARY_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    S,
    A,
    B,
    C,
    D,
    E,
}

impl Tier {
    pub const ALL: [Self; 6] =
        [Self::S, Self::A, Self::B, Self::C, Self::D, Self::E];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
        }
    }
}

/// Tiers by percentile of the overall score among trackers with enough weight;
/// `None` is unranked. Output order matches input.
#[must_use]
pub fn assign_tiers(
    overall: &[AxisScore],
    cfg: &ScoringConfig,
) -> Vec<Option<Tier>> {
    let eligible = |s: &AxisScore| s.effective_weight >= cfg.rank_min_weight;
    let ranked: Vec<f64> =
        overall.iter().filter(|s| eligible(s)).map(|s| s.score).collect();
    let n = to_f64(ranked.len());

    overall
        .iter()
        .map(|s| {
            if !eligible(s) {
                return None;
            }
            let higher = ranked.iter().filter(|o| **o > s.score).count();
            let p = to_f64(higher) / n;
            let mut cumulative = 0.0;
            for (tier, share) in Tier::ALL.into_iter().zip(cfg.tier_shares) {
                cumulative += share;
                if p < cumulative - BOUNDARY_EPSILON {
                    return Some(tier);
                }
            }
            Some(Tier::E)
        })
        .collect()
}

fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}
