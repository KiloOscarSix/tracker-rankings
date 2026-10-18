use rankings_core::aggregate::{AxisRating, aggregate};
use rankings_core::axis::Axis;
use rankings_core::config::ScoringConfig;
use rankings_core::error::InvalidScore;
use rankings_core::score::{Rating, Score};
use rankings_core::weight::Voter;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

fn rate(
    tracker_id: i32,
    axis: Axis,
    score: i16,
    voter: Voter,
) -> Result<AxisRating, InvalidScore> {
    Ok(AxisRating {
        tracker_id,
        axis,
        rating: Rating { score: Score::try_from(score)?, voter },
    })
}

#[test]
fn aggregate_uses_each_axis_mean_and_skips_unrated_pairs() {
    let ratings = [
        rate(1, Axis::Overall, 5, Voter::Anonymous).unwrap(),
        rate(1, Axis::Overall, 3, Voter::Account { provider_age_days: 400 })
            .unwrap(),
        rate(2, Axis::Overall, 4, Voter::Account { provider_age_days: 10 }).unwrap(),
        rate(2, Axis::Content, 2, Voter::Account { provider_age_days: 400 })
            .unwrap(),
    ];

    let out = aggregate(&ratings, &ScoringConfig::default());

    let keys: Vec<_> = out.iter().map(|s| (s.tracker_id, s.axis)).collect();
    assert_eq!(keys, [(1, Axis::Overall), (2, Axis::Overall), (2, Axis::Content)]);

    let m = 9.7 / 2.5;
    let [t1, t2, t2_content]: [_; 3] = out.try_into().unwrap();
    let [t1, t2, t2_content] = [t1.score, t2.score, t2_content.score];

    let expected = 5.0_f64.mul_add(m, 6.5) / 6.7;
    assert!(close(t1.score, expected), "{} vs {expected}", t1.score);
    assert!(close(t1.effective_weight, 1.7), "{}", t1.effective_weight);
    assert_eq!(t1.vote_count, 2);

    let expected = 5.0_f64.mul_add(m, 3.2) / 5.8;
    assert!(close(t2.score, expected), "{} vs {expected}", t2.score);
    assert!(close(t2.effective_weight, 0.8), "{}", t2.effective_weight);
    assert_eq!(t2.vote_count, 1);

    assert!(close(t2_content.score, 2.0), "{}", t2_content.score);
    let weight = t2_content.effective_weight;
    assert!(close(weight, 1.0), "{weight}");
    assert_eq!(t2_content.vote_count, 1);
}
