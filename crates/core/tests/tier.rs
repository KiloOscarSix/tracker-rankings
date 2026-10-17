use rankings_core::config::ScoringConfig;
use rankings_core::score::AxisScore;
use rankings_core::tier::{Tier, assign_tiers};

const fn scored(score: f64, effective_weight: f64) -> AxisScore {
    AxisScore { score, effective_weight, vote_count: 0 }
}

#[test]
fn trackers_without_enough_weight_are_unranked() {
    let overall = [scored(4.9, 2.9), scored(3.0, 3.0)];
    let tiers = assign_tiers(&overall, &ScoringConfig::default());
    assert_eq!(tiers, [None, Some(Tier::S)]);
}

#[test]
fn twenty_trackers_split_by_default_shares() {
    let overall: Vec<AxisScore> =
        (0..20_u8).map(|i| scored(f64::from(i), 10.0)).collect();
    let tiers = assign_tiers(&overall, &ScoringConfig::default());

    let mut counts = [0_usize; 6];
    for tier in tiers.iter().flatten() {
        let index = Tier::ALL.iter().position(|t| t == tier).unwrap();
        counts[index] += 1;
    }
    assert_eq!(counts, [1, 3, 5, 5, 4, 2]);
    assert_eq!(tiers[19], Some(Tier::S));
    assert_eq!(tiers[0], Some(Tier::E));
}

#[test]
fn tied_scores_share_a_tier() {
    let scores = [5.0, 5.0, 4.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0, 0.0];
    let overall: Vec<AxisScore> = scores.iter().map(|s| scored(*s, 10.0)).collect();
    let tiers = assign_tiers(&overall, &ScoringConfig::default());

    assert_eq!(tiers[0], tiers[1]);
    assert_eq!(tiers[3], tiers[4]);
    assert_eq!(tiers[4], tiers[5]);
    assert_eq!(tiers[0], Some(Tier::S));
    assert_eq!(tiers[1], Some(Tier::S));
}
