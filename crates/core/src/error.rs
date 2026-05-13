#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("upstream type token {0:?} has no category mapping")]
pub struct UnknownCategoryToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a category key")]
pub struct UnknownCategoryKey(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("rating score {0} is outside 1..=5")]
pub struct InvalidScore(pub i16);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("{name} must be finite and > 0, got {value}")]
    NotPositive { name: &'static str, value: f64 },
    #[error("tier shares must be >= 0 and sum to 1, got {0:?}")]
    TierShares([f64; 6]),
    #[error("bradley_terry.max_iterations must be > 0")]
    ZeroIterations,
}

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum PairwiseError {
    #[error("matchup references item {index}, but only {len} items exist")]
    OutOfRange { index: usize, len: usize },
    #[error("matchup has the same winner and loser ({0})")]
    SelfMatch(usize),
    #[error("matchup weight must be finite and > 0, got {0}")]
    InvalidWeight(f64),
}
