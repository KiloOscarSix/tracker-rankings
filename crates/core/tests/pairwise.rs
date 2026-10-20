use rankings_core::config::ScoringConfig;
use rankings_core::pairwise::{PairwiseVote, rank};
use rankings_core::weight::Voter;

const ACCOUNT: Voter = Voter::Account { provider_age_days: 365 };

const fn vote(winner_id: i32, loser_id: i32, voter: Voter) -> PairwiseVote {
    PairwiseVote { winner_id, loser_id, voter }
}

#[test]
fn no_votes_rank_nothing() {
    let ranking = rank(&[], &ScoringConfig::default()).unwrap();

    assert_eq!(ranking.strengths, []);
    assert!(ranking.converged);
}

#[test]
fn strengths_come_back_by_tracker_id_with_counts() {
    let votes =
        [vote(30, 10, ACCOUNT), vote(30, 20, ACCOUNT), vote(10, 20, ACCOUNT)];

    let ranking = rank(&votes, &ScoringConfig::default()).unwrap();

    let ids: Vec<i32> = ranking.strengths.iter().map(|s| s.tracker_id).collect();
    assert_eq!(ids, [10, 20, 30]);
    let counts: Vec<u32> = ranking.strengths.iter().map(|s| s.vote_count).collect();
    assert_eq!(counts, [2, 2, 2]);
    let [s10, s20, s30] = [0, 1, 2].map(|i| ranking.strengths[i].strength);
    assert!(s30 > s10 && s10 > s20, "{s30} {s10} {s20}");
    assert!([s10, s20, s30].iter().all(|s| s.is_finite() && *s > 0.0));
}

#[test]
fn account_win_outweighs_anonymous_loss() {
    let votes = [vote(1, 2, Voter::Anonymous), vote(2, 1, ACCOUNT)];

    let ranking = rank(&votes, &ScoringConfig::default()).unwrap();

    let [one, two] = [0, 1].map(|i| ranking.strengths[i]);
    assert_eq!((one.tracker_id, two.tracker_id), (1, 2));
    assert!(two.strength > one.strength, "{} {}", two.strength, one.strength);
    assert_eq!((one.vote_count, two.vote_count), (2, 2));
}
