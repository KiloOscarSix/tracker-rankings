use std::str::FromStr;

use crate::error::UnknownMembershipStatus;

/// Self-declared membership; keys match the `membership_status` Postgres enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MembershipStatus {
    Current,
    Former,
}

impl MembershipStatus {
    pub const ALL: [Self; 2] = [Self::Current, Self::Former];

    /// The value stored in the database.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Former => "former",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Current => "Current member",
            Self::Former => "Former member",
        }
    }
}

impl FromStr for MembershipStatus {
    type Err = UnknownMembershipStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.key() == s)
            .ok_or_else(|| UnknownMembershipStatus(s.to_owned()))
    }
}
