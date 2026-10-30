pub mod slug;

use rankings_core::category::Category;
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
use topcoat::router::route;

use crate::app::compare::compare_href;
use crate::auth::voter::{current_voter, voter_for_write};
use crate::context::{db, score_signal};
use crate::queries::pairwise::{pair_ids, upsert_pairwise_vote};
use crate::queries::votes::is_member;
use crate::votes::CompareForm;

#[route(POST "./compare")]
pub(crate) async fn compare_vote(
    cx: &Cx,
    Form(form): Form<CompareForm>,
) -> Result<SeeOther> {
    if form.a == form.b {
        return Err(bad_request("pick two different trackers").into());
    }
    let loser = if form.winner == form.a {
        &form.b
    } else if form.winner == form.b {
        &form.a
    } else {
        return Err(bad_request("the winner must be one of the pair").into());
    };
    let category = form
        .category
        .as_deref()
        .map(str::parse::<Category>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?;
    let pool = db(cx)?;
    let signal = score_signal(cx)?;
    let pair = pair_ids(pool, &form.winner, loser).await?.ok_or_not_found()?;

    let voter = if pair.share_category {
        voter_for_write(cx).await?
    } else {
        let voter = current_voter(cx).await?.ok_or_forbidden()?;
        if !is_member(pool, voter, pair.winner_id).await?
            || !is_member(pool, voter, pair.loser_id).await?
        {
            return Err(forbidden().into());
        }
        voter
    };
    upsert_pairwise_vote(pool, voter, pair.winner_id, pair.loser_id).await?;
    signal.notify();
    Ok(see_other(compare_href(category) + "#matchup"))
}
