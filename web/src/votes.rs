use rankings_core::axis::Axis;
use rankings_core::membership::MembershipStatus;
use rankings_core::score::Score;
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
use topcoat::router::{RouterBuilder, path_param, route};

use crate::auth::anon::{clear_anon_cookie, existing_anon_voter};
use crate::auth::sessions::{current_account, sign_out};
use crate::auth::voter::{current_voter, voter_for_write};
use crate::context::{db, score_signal};
use crate::queries::votes::{
    delete_account,
    delete_anon_identity,
    is_member,
    listed_tracker_id,
    remove_membership,
    set_membership,
    upsert_ratings,
};

path_param!(pub slug);

/// The membership form value that removes the membership.
const REMOVE: &str = "none";

#[derive(Debug, serde::Deserialize)]
pub struct MembershipForm {
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct RatingsForm {
    pub overall: Option<i16>,
    pub content: Option<i16>,
    pub community: Option<i16>,
    pub retention: Option<i16>,
    pub longevity: Option<i16>,
}

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.route(membership).route(ratings).route(delete_my_data)
}

#[route(POST "/vote/{slug}/membership")]
pub async fn membership(
    cx: &Cx,
    Form(form): Form<MembershipForm>,
) -> Result<SeeOther> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let tracker = listed_tracker_id(pool, slug).await?.ok_or_not_found()?;

    if form.status == REMOVE {
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

#[route(POST "/vote/{slug}/ratings")]
pub async fn ratings(cx: &Cx, Form(form): Form<RatingsForm>) -> Result<SeeOther> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let tracker = listed_tracker_id(pool, slug).await?.ok_or_not_found()?;

    let picks = [
        (Axis::Overall, form.overall),
        (Axis::Content, form.content),
        (Axis::Community, form.community),
        (Axis::Retention, form.retention),
        (Axis::Longevity, form.longevity),
    ];
    let mut scores: Vec<(Axis, Score)> = Vec::new();
    for (axis, raw) in picks {
        if let Some(raw) = raw {
            let score =
                Score::try_from(raw).map_err(|e| bad_request(e.to_string()))?;
            scores.push((axis, score));
        }
    }
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

#[route(POST "/me/delete")]
pub async fn delete_my_data(cx: &Cx) -> Result<SeeOther> {
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let account = current_account(cx).await?;
    if let Some(account) = &account {
        sign_out(cx, pool).await?;
        delete_account(pool, account.id).await?;
    }
    let anon = existing_anon_voter(cx).await?;
    if let Some(voter) = anon {
        delete_anon_identity(pool, voter).await?;
    }
    if account.is_some() || anon.is_some() {
        signal.notify();
    }
    clear_anon_cookie(cx);
    Ok(see_other("/me?deleted=1"))
}
