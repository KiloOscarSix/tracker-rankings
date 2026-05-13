use std::collections::BTreeSet;
use std::str::FromStr;

use crate::error::{UnknownCategoryKey, UnknownCategoryToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    MoviesTv,
    Music,
    Books,
    Anime,
    GamesSoftware,
    General,
    ELearning,
    Adult,
    Regional,
}

impl Category {
    pub const ALL: [Self; 9] = [
        Self::MoviesTv,
        Self::Music,
        Self::Books,
        Self::Anime,
        Self::GamesSoftware,
        Self::General,
        Self::ELearning,
        Self::Adult,
        Self::Regional,
    ];

    /// The value stored in the database.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::MoviesTv => "movies-tv",
            Self::Music => "music",
            Self::Books => "books",
            Self::Anime => "anime",
            Self::GamesSoftware => "games-software",
            Self::General => "general",
            Self::ELearning => "e-learning",
            Self::Adult => "adult",
            Self::Regional => "regional",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MoviesTv => "Movies/TV",
            Self::Music => "Music",
            Self::Books => "Books/Audiobooks",
            Self::Anime => "Anime",
            Self::GamesSoftware => "Games/Software",
            Self::General => "General",
            Self::ELearning => "E-Learning",
            Self::Adult => "Adult",
            Self::Regional => "Regional",
        }
    }

    /// Maps a single upstream `type` token, ignoring case and surrounding
    /// whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownCategoryToken`] when the token has no mapping.
    pub fn from_token(token: &str) -> Result<Self, UnknownCategoryToken> {
        match token.trim().to_ascii_lowercase().as_str() {
            "movie" | "movies" | "tv" | "animated" | "cartoon" | "classic"
            | "vintage" | "old/obscure" | "arthouse/obscure" | "discs only"
            | "fanres" | "documentaries" | "comedy" | "theatre" | "sports"
            | "motorsports" | "racing" => Ok(Self::MoviesTv),
            "music" | "audio" | "music production" | "podcasts" => Ok(Self::Music),
            "books" | "ebooks" | "audiobooks" | "comics" | "magazines" => {
                Ok(Self::Books)
            },
            "anime" | "manga" => Ok(Self::Anime),
            "games" | "pc" | "console" | "mac software" | "0day" | "3d printing"
            | "cg" | "vfx" => Ok(Self::GamesSoftware),
            "general" | "scene" | "requests" => Ok(Self::General),
            "e-learning" | "business" | "self-help" | "pick-up"
            | "premium videos" => Ok(Self::ELearning),
            "porn" | "xxx" | "jav" | "hentai" => Ok(Self::Adult),
            "asian" | "brazilian" | "castellano" | "chinese" | "european"
            | "french" | "german" | "hungarian" | "indian" | "italian"
            | "latino" | "malaysian" | "nordic" | "portuguese" | "romanian"
            | "swedish" | "turkish" | "ukrainian" => Ok(Self::Regional),
            _ => Err(UnknownCategoryToken(token.trim().to_owned())),
        }
    }

    /// Upstream `type` is comma-separated free text, e.g. `"Movie, TV, General"`.
    /// Returns the deduplicated categories in declaration order.
    ///
    /// # Errors
    ///
    /// Returns the first [`UnknownCategoryToken`] encountered.
    pub fn normalise(raw_type: &str) -> Result<Vec<Self>, UnknownCategoryToken> {
        let categories = raw_type
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(Self::from_token)
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok(categories.into_iter().collect())
    }
}

impl FromStr for Category {
    type Err = UnknownCategoryKey;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|category| category.key() == s)
            .ok_or_else(|| UnknownCategoryKey(s.to_owned()))
    }
}
