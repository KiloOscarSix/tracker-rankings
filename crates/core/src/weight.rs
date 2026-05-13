use crate::config::VoteWeights;

/// Who cast a vote, as seen at aggregation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voter {
    Account { provider_age_days: u32 },
    Anonymous,
}

impl VoteWeights {
    #[must_use]
    pub const fn weight(&self, voter: Voter) -> f64 {
        match voter {
            Voter::Account { provider_age_days }
                if provider_age_days < self.young_account_days =>
            {
                self.young_account
            },
            Voter::Account { .. } => self.account,
            Voter::Anonymous => self.anonymous,
        }
    }
}
