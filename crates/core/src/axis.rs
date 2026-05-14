use std::str::FromStr;

use crate::error::UnknownAxisKey;

/// A rating axis; keys match the `rating_axis` Postgres enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    Overall,
    Content,
    Community,
    Retention,
    Longevity,
}

impl Axis {
    pub const ALL: [Self; 5] = [
        Self::Overall,
        Self::Content,
        Self::Community,
        Self::Retention,
        Self::Longevity,
    ];

    /// The value stored in the database.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Overall => "overall",
            Self::Content => "content",
            Self::Community => "community",
            Self::Retention => "retention",
            Self::Longevity => "longevity",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overall => "Overall",
            Self::Content => "Content",
            Self::Community => "Community",
            Self::Retention => "Retention/ratio ease",
            Self::Longevity => "Longevity/stability",
        }
    }
}

impl FromStr for Axis {
    type Err = UnknownAxisKey;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|axis| axis.key() == s)
            .ok_or_else(|| UnknownAxisKey(s.to_owned()))
    }
}
