use rankings_core::bradley_terry::{Matchup, fit};
use rankings_core::config::BradleyTerryConfig;
use rankings_core::error::PairwiseError;

#[test]
fn recovers_known_order_and_ratios() {
    let truth = [8.0, 4.0, 2.0, 1.0];
    let mut matchups = Vec::new();
    for i in 0..truth.len() {
        for j in (i + 1)..truth.len() {
            let games = 100.0;
            let wins_i = games * truth[i] / (truth[i] + truth[j]);
            matchups.push(Matchup { winner: i, loser: j, weight: wins_i });
            matchups.push(Matchup { winner: j, loser: i, weight: games - wins_i });
        }
    }
    let cfg = BradleyTerryConfig {
        prior_games: 0.1,
        max_iterations: 1_000_000,
        tolerance: 1e-9,
    };

    let fit = fit(truth.len(), &matchups, &cfg).unwrap();

    assert!(fit.converged);
    let s = &fit.strengths;
    assert!(s.windows(2).all(|w| w[0] > w[1]), "not decreasing: {s:?}");
    for i in 0..truth.len() {
        let got = s[i] / s[3];
        let want = truth[i] / truth[3];
        assert!(
            ((got / want) - 1.0).abs() < 0.02,
            "item {i}: fitted ratio {got}, true ratio {want}"
        );
    }
}

#[test]
fn zero_win_and_unvoted_items_stay_positive() {
    let matchups = [Matchup { winner: 0, loser: 1, weight: 3.0 }];

    let fit = fit(3, &matchups, &BradleyTerryConfig::default()).unwrap();

    assert!(fit.converged);
    let s = &fit.strengths;
    assert!(s.iter().all(|p| p.is_finite() && *p > 0.0), "{s:?}");
    assert!(s[0] > s[1]);
    assert!((s[2] - 1.0).abs() < 1e-9, "unvoted item moved: {}", s[2]);
}

#[test]
fn invalid_matchups_are_rejected() {
    let cfg = BradleyTerryConfig::default();
    let run =
        |winner, loser, weight| fit(2, &[Matchup { winner, loser, weight }], &cfg);

    assert_eq!(run(5, 0, 1.0), Err(PairwiseError::OutOfRange { index: 5, len: 2 }));
    assert_eq!(run(1, 1, 1.0), Err(PairwiseError::SelfMatch(1)));
    assert!(matches!(run(0, 1, 0.0), Err(PairwiseError::InvalidWeight(_))));
    assert!(matches!(run(0, 1, f64::NAN), Err(PairwiseError::InvalidWeight(_))));
}
