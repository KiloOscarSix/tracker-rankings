use rankings_core::axis::Axis;
use rankings_core::category::Category;
use rankings_core::membership::MembershipStatus;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::{
    RouterErrorExt,
    SeeOther,
    bad_request,
    forbidden,
    see_other,
};
use topcoat::router::{module_param, path_param, route};

use crate::app::rankings::rankings_href;
use crate::auth::voter::{current_voter, voter_for_write};
use crate::context::{db, score_signal};
use crate::queries::votes::{
    is_member,
    listed_tracker_id,
    remove_membership,
    set_membership,
    upsert_ratings,
};
use crate::votes::{
    MembershipForm,
    QuickVoteForm,
    REMOVE,
    RatingsForm,
    confirmed,
    parse_scores,
};

module_param!(pub slug);

#[route(POST)]
pub(crate) async fn quick_vote(
    cx: &Cx,
    Form(form): Form<QuickVoteForm>,
) -> Result<SeeOther> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let tracker = listed_tracker_id(pool, slug).await?.ok_or_not_found()?;

    let status = form
        .status
        .parse::<MembershipStatus>()
        .map_err(|e| bad_request(e.to_string()))?;
    let scores = parse_scores([
        (Axis::Overall, form.overall),
        (Axis::Content, form.content),
        (Axis::Community, form.community),
        (Axis::Retention, form.retention),
        (Axis::Longevity, form.longevity),
    ])?;
    let category = form
        .category
        .as_deref()
        .filter(|c| !c.is_empty())
        .map(str::parse::<Category>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?;
    let sort = form
        .sort
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::parse::<Axis>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?
        .unwrap_or(Axis::Overall);

    let voter = voter_for_write(cx).await?;
    set_membership(pool, voter, tracker, status).await?;
    if !scores.is_empty() {
        upsert_ratings(pool, voter, tracker, &scores).await?;
        signal.notify();
    }
    let q = form.q.as_deref().unwrap_or_default();
    Ok(see_other(format!(
        "{}#tracker-{slug}",
        rankings_href(category, sort, q, Some(slug))
    )))
}

#[route(POST "./membership")]
pub(crate) async fn membership(
    cx: &Cx,
    Form(form): Form<MembershipForm>,
) -> Result<SeeOther> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let tracker = listed_tracker_id(pool, slug).await?.ok_or_not_found()?;

    if form.status == REMOVE {
        if !confirmed(form.confirm.as_deref()) {
            return Err(bad_request("confirm removing the membership").into());
        }
        if let Some(voter) = current_voter(cx).await? {
            remove_membership(pool, voter, tracker).await?;
            signal.notify();
        }
    } else {
        let status = form
            .status
            .parse::<MembershipStatus>()
            .map_err(|e| bad_request(e.to_string()))?;
        let voter = voter_for_write(cx).await?;
        set_membership(pool, voter, tracker, status).await?;
    }
    Ok(see_other(format!("/t/{slug}#vote")))
}

#[route(POST "./ratings")]
pub(crate) async fn ratings(
    cx: &Cx,
    Form(form): Form<RatingsForm>,
) -> Result<SeeOther> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let tracker = listed_tracker_id(pool, slug).await?.ok_or_not_found()?;

    let scores = parse_scores([
        (Axis::Overall, form.overall),
        (Axis::Content, form.content),
        (Axis::Community, form.community),
        (Axis::Retention, form.retention),
        (Axis::Longevity, form.longevity),
    ])?;
    if scores.is_empty() {
        return Err(bad_request("rate at least one axis").into());
    }

    let voter = current_voter(cx).await?.ok_or_forbidden()?;
    if !is_member(pool, voter, tracker).await? {
        return Err(forbidden().into());
    }
    upsert_ratings(pool, voter, tracker, &scores).await?;
    signal.notify();
    Ok(see_other(format!("/t/{slug}#vote")))
}
