use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::ImportError;

const SEED_FILE: &str = "trackers.json";
const SOURCE_FILE: &str = "SOURCE";

/// Upstream `src/data/trackers.json`; shapes follow upstream `src/types/index.ts`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seed {
    #[serde(rename = "trackerInfo")]
    pub trackers: BTreeMap<String, TrackerInfo>,
    #[serde(rename = "routeInfo")]
    pub routes: BTreeMap<String, BTreeMap<String, RouteInfo>>,
    #[serde(rename = "unlockInviteClass")]
    pub unlocks: BTreeMap<String, Unlock>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerInfo {
    pub abbr: String,
    #[serde(rename = "type")]
    pub raw_type: String,
    pub country: String,
    pub birthdate: Birthdate,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Birthdate {
    Year(i16),
    Text(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteInfo {
    pub days: Option<i32>,
    pub reqs: String,
    pub active: String,
    pub updated: String,
}

/// Days to reach invite rank, and the rank requirements.
#[derive(Debug, Deserialize)]
pub struct Unlock(pub Option<i32>, pub String);

/// The vendored `SOURCE` attribution file.
#[derive(Debug)]
pub struct Source {
    pub commit: String,
}

impl Birthdate {
    pub(crate) fn year(&self, tracker: &str) -> Result<Option<i16>, ImportError> {
        match self {
            Self::Year(year) => Ok(Some(*year)),
            Self::Text(text) if text.is_empty() => Ok(None),
            Self::Text(text) => {
                text.parse().map(Some).map_err(|source| ImportError::InvalidYear {
                    tracker: tracker.to_owned(),
                    value: text.clone(),
                    source,
                })
            },
        }
    }
}

impl Source {
    pub fn parse(text: &str) -> Result<Self, ImportError> {
        let commit = text
            .lines()
            .find_map(|line| line.strip_prefix("commit:"))
            .map(str::trim)
            .ok_or(ImportError::SourceKey("commit"))?;
        Ok(Self { commit: commit.to_owned() })
    }
}

/// Reads `trackers.json` and `SOURCE` from a vendored seed directory.
pub fn load(dir: &Path) -> Result<(Seed, Source), ImportError> {
    let seed = serde_json::from_str(&read(&dir.join(SEED_FILE))?)?;
    let source = Source::parse(&read(&dir.join(SOURCE_FILE))?)?;
    Ok((seed, source))
}

fn read(path: &Path) -> Result<String, ImportError> {
    fs::read_to_string(path)
        .map_err(|source| ImportError::Read { path: path.to_owned(), source })
}
