use ladder_core::config::VoteWeights;
use ladder_core::error::InvalidScore;
use ladder_core::score::{Rating, Score, Tally, bayesian, global_mean, tally};
use ladder_core::weight::Voter;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

fn rating(score: i16, voter: Voter) -> Result<Rating, InvalidScore> {
    Ok(Rating { score: Score::try_from(score)?, voter })
}

#[test]
fn no_votes_score_is_the_mean() {
    let axis = bayesian(&tally(&[], &VoteWeights::default()), 3.4, 5.0);
    assert!(close(axis.score, 3.4), "{} vs 3.4", axis.score);
    assert!(close(axis.effective_weight, 0.0), "{}", axis.effective_weight);
    assert_eq!(axis.vote_count, 0);
}

#[test]
fn one_full_vote_moves_score_by_its_weight() {
    let ratings = [rating(5, Voter::Account { provider_age_days: 400 }).unwrap()];
    let axis = bayesian(&tally(&ratings, &VoteWeights::default()), 3.0, 5.0);
    let expected = (15.0 + 5.0) / 6.0;
    assert!(close(axis.score, expected), "{} vs {expected}", axis.score);
    assert!(close(axis.effective_weight, 1.0), "{}", axis.effective_weight);
    assert_eq!(axis.vote_count, 1);
}

#[test]
fn weight_classes_follow_provider_age() {
    let weights = VoteWeights::default();
    let cases = [
        (Voter::Account { provider_age_days: 0 }, 0.8),
        (Voter::Account { provider_age_days: 29 }, 0.8),
        (Voter::Account { provider_age_days: 30 }, 1.0),
        (Voter::Anonymous, 0.7),
    ];
    for (voter, expected) in cases {
        let w = weights.weight(voter);
        assert!(close(w, expected), "{voter:?}: {w} vs {expected}");
    }

    let ratings = [
        rating(4, Voter::Anonymous).unwrap(),
        rating(2, Voter::Account { provider_age_days: 10 }).unwrap(),
        rating(5, Voter::Account { provider_age_days: 365 }).unwrap(),
    ];
    let t = tally(&ratings, &weights);
    let parts = [0.7 * 4.0, 0.8 * 2.0, 1.0 * 5.0];
    let expected_sum: f64 = parts.iter().sum();
    assert!(close(t.weight, 2.5), "{}", t.weight);
    assert!(close(t.weighted_sum, expected_sum), "{}", t.weighted_sum);
    assert_eq!(t.count, 3);
}

#[test]
fn global_mean_weights_trackers_and_falls_back() {
    let empty = global_mean(&[], 3.0);
    assert!(close(empty, 3.0), "{empty}");

    let tallies = [Tally { weight: 1.0, weighted_sum: 5.0, count: 1 }, Tally {
        weight: 3.0,
        weighted_sum: 3.0,
        count: 3,
    }];
    let mean = global_mean(&tallies, 3.0);
    assert!(close(mean, 2.0), "{mean}");
}

#[test]
fn score_rejects_values_outside_one_to_five() {
    assert_eq!(Score::try_from(0), Err(InvalidScore(0)));
    assert_eq!(Score::try_from(6), Err(InvalidScore(6)));
    assert_eq!(Score::try_from(-1), Err(InvalidScore(-1)));
    assert_eq!(Score::try_from(1).unwrap().get(), 1);
    assert_eq!(Score::try_from(5).unwrap().get(), 5);
}
