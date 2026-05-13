use crate::error::ConfigError;

/// Every tunable of the scoring pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringConfig {
    pub weights: VoteWeights,
    /// `C` in the Bayesian average: the prior's pseudo-weight.
    pub confidence: f64,
    /// `m` for an axis that has no votes on any tracker.
    pub fallback_mean: f64,
    /// Effective weight a tracker's overall score needs before it gets a tier.
    pub rank_min_weight: f64,
    /// Share of ranked trackers in each tier, S first.
    pub tier_shares: [f64; 6],
    pub bradley_terry: BradleyTerryConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoteWeights {
    pub account: f64,
    pub young_account: f64,
    /// Provider accounts younger than this many days get `young_account`.
    pub young_account_days: u32,
    pub anonymous: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BradleyTerryConfig {
    /// Virtual wins and losses each item gets against a strength-1 pseudo-item.
    pub prior_games: f64,
    pub max_iterations: u32,
    /// Stop once no strength changes by more than this relative amount.
    pub tolerance: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weights: VoteWeights::default(),
            confidence: 5.0,
            fallback_mean: 3.0,
            rank_min_weight: 3.0,
            tier_shares: [0.05, 0.15, 0.25, 0.25, 0.20, 0.10],
            bradley_terry: BradleyTerryConfig::default(),
        }
    }
}

impl Default for VoteWeights {
    fn default() -> Self {
        Self {
            account: 1.0,
            young_account: 0.8,
            young_account_days: 30,
            anonymous: 0.7,
        }
    }
}

impl Default for BradleyTerryConfig {
    fn default() -> Self {
        Self { prior_games: 1.0, max_iterations: 10_000, tolerance: 1e-8 }
    }
}

impl ScoringConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        positive("weights.account", self.weights.account)?;
        positive("weights.young_account", self.weights.young_account)?;
        positive("weights.anonymous", self.weights.anonymous)?;
        positive("confidence", self.confidence)?;
        positive("fallback_mean", self.fallback_mean)?;
        positive("rank_min_weight", self.rank_min_weight)?;
        positive("bradley_terry.prior_games", self.bradley_terry.prior_games)?;
        positive("bradley_terry.tolerance", self.bradley_terry.tolerance)?;
        if self.bradley_terry.max_iterations == 0 {
            return Err(ConfigError::ZeroIterations);
        }
        let shares = self.tier_shares;
        let sum: f64 = shares.iter().sum();
        if shares.iter().any(|s| !s.is_finite() || *s < 0.0)
            || (sum - 1.0).abs() > 1e-9
        {
            return Err(ConfigError::TierShares(shares));
        }
        Ok(())
    }
}

fn positive(name: &'static str, value: f64) -> Result<(), ConfigError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(ConfigError::NotPositive { name, value })
    }
}
