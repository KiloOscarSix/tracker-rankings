use std::str::FromStr;

use crate::error::UnknownProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Discord,
    Reddit,
}

impl Provider {
    pub const ALL: [Self; 2] = [Self::Discord, Self::Reddit];

    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Discord => "discord",
            Self::Reddit => "reddit",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Discord => "Discord",
            Self::Reddit => "Reddit",
        }
    }
}

impl FromStr for Provider {
    type Err = UnknownProvider;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|provider| provider.key() == s)
            .ok_or_else(|| UnknownProvider(s.to_owned()))
    }
}
