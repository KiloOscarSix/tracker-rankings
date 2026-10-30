use rankings_core::axis::Axis;
use rankings_core::score::Score;
use topcoat::Result;
use topcoat::router::error::bad_request;

/// The membership form value that removes the membership.
pub const REMOVE: &str = "none";

pub const CONFIRMED: &str = "yes";

#[derive(Debug, serde::Deserialize)]
pub struct MembershipForm {
    pub status: String,
    pub confirm: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfirmForm {
    pub confirm: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RatingsForm {
    pub overall: Option<i16>,
    pub content: Option<i16>,
    pub community: Option<i16>,
    pub retention: Option<i16>,
    pub longevity: Option<i16>,
}

#[derive(Debug, serde::Deserialize)]
pub struct QuickVoteForm {
    pub status: String,
    pub overall: Option<i16>,
    pub content: Option<i16>,
    pub community: Option<i16>,
    pub retention: Option<i16>,
    pub longevity: Option<i16>,
    pub category: Option<String>,
    pub sort: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CompareForm {
    pub a: String,
    pub b: String,
    pub winner: String,
    pub category: Option<String>,
}

pub(crate) fn confirmed(confirm: Option<&str>) -> bool {
    confirm == Some(CONFIRMED)
}

pub(crate) fn parse_scores(
    picks: [(Axis, Option<i16>); 5],
) -> Result<Vec<(Axis, Score)>> {
    let mut scores: Vec<(Axis, Score)> = Vec::new();
    for (axis, raw) in picks {
        if let Some(raw) = raw {
            let score =
                Score::try_from(raw).map_err(|e| bad_request(e.to_string()))?;
            scores.push((axis, score));
        }
    }
    Ok(scores)
}
