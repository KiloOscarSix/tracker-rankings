use rankings_core::axis::Axis;
use rankings_core::category::Category;
use rankings_core::membership::MembershipStatus;
use rankings_core::score::{AxisScore, Score};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::{RouterErrorExt, bad_request};
use topcoat::router::{page, path_param, query_params};
use topcoat::view::{View, attributes, component, view};

use crate::auth::sessions::current_account;
use crate::auth::voter::current_voter;
use crate::components::button::button;
use crate::components::choice::choice;
use crate::components::score::score_cell;
use crate::context::{db, scoring};
use crate::pages::rankings::rankings_href;
use crate::queries::trackers::{tracker_by_slug, tracker_scores};
use crate::queries::votes::voter_tracker;
use crate::rankings::axis_index;

path_param!(pub slug);

pub(crate) struct VotePanel {
    pub slug: String,
    pub categories: String,
    pub axis_scores: Vec<(&'static str, Option<AxisScore>)>,
    pub status_options: Vec<(&'static str, &'static str, bool)>,
    pub rating_rows: Vec<(&'static str, &'static str, Option<u8>)>,
    pub saved: bool,
    pub vote_note: Option<String>,
    pub category: Option<Category>,
    pub sort: Axis,
    pub q: String,
}

pub(crate) struct PanelContext {
    pub category: Option<Category>,
    pub sort: Axis,
    pub q: String,
    pub saved: bool,
    pub vote_note: Option<String>,
}

impl VotePanel {
    /// `scores` and `ratings` are indexed by `axis_index`.
    pub(crate) fn new(
        slug: String,
        categories: &[Category],
        scores: [Option<AxisScore>; 5],
        mine: Option<(MembershipStatus, [Option<Score>; 5])>,
        context: PanelContext,
    ) -> Self {
        let PanelContext { category, sort, q, saved, vote_note } = context;
        let categories =
            categories.iter().map(|c| c.label()).collect::<Vec<_>>().join(", ");
        let my_status = mine.as_ref().map(|(status, _)| *status);
        let status_options = MembershipStatus::ALL
            .into_iter()
            .map(|status| (status.key(), status.label(), my_status == Some(status)))
            .collect();
        let rating_rows = Axis::ALL
            .into_iter()
            .map(|axis| {
                let current = mine
                    .as_ref()
                    .and_then(|(_, ratings)| {
                        ratings.get(axis_index(axis)).copied().flatten()
                    })
                    .map(Score::get);
                (axis.key(), axis.label(), current)
            })
            .collect();
        let axis_scores = Axis::ALL
            .into_iter()
            .filter(|axis| *axis != Axis::Overall)
            .map(|axis| {
                (axis.label(), scores.get(axis_index(axis)).copied().flatten())
            })
            .collect();
        Self {
            slug,
            categories,
            axis_scores,
            status_options,
            rating_rows,
            saved,
            vote_note,
            category,
            sort,
            q,
        }
    }
}

pub(crate) fn vote_note(signed_in: bool, anonymous_weight: f64) -> Option<String> {
    (!signed_in).then(|| {
        format!(
            "Anonymous votes count {anonymous_weight}\u{d7}; sign in for full weight."
        )
    })
}

#[component]
pub(crate) async fn vote_panel(panel: VotePanel) -> Result<impl View> {
    let VotePanel {
        slug,
        categories,
        axis_scores,
        status_options,
        rating_rows,
        saved,
        vote_note,
        category,
        sort,
        q,
    } = panel;

    Ok(view! {
        <div data-vote-panel="" class="space-y-4">
            <p class="text-xs text-muted-foreground xl:hidden">(categories)</p>
            <dl class="grid grid-cols-2 gap-3 sm:grid-cols-4 lg:hidden">
                for (label, score) in axis_scores {
                    <div>
                        <dt class="text-xs text-muted-foreground">(label)</dt>
                        <dd>score_cell(score: score)</dd>
                    </div>
                }
            </dl>
            <form method="post" action=(format!("/vote/{slug}")) class="space-y-4">
                <fieldset>
                    <legend class="mb-2 text-sm font-medium">"Your membership"</legend>
                    <div class="flex flex-wrap gap-2">
                        for (key, label, checked) in status_options {
                            choice(
                                attrs: attributes! {
                                    name="status"
                                    value=(key)
                                    checked=(checked)
                                    required=""
                                },
                                (label)
                            )
                        }
                    </div>
                </fieldset>
                <fieldset>
                    <legend class="mb-2 text-sm font-medium">
                        "Your ratings "
                        <span class="font-normal text-muted-foreground">
                            "(optional, 1 = worst, 5 = best)"
                        </span>
                    </legend>
                    <div class="grid w-fit gap-x-12 gap-y-2 md:grid-cols-2">
                        for (key, label, current) in rating_rows {
                            <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
                                <span id=(format!("rate-{slug}-{key}")) class="w-40 text-sm">
                                    (label)
                                </span>
                                <div
                                    role="radiogroup"
                                    aria-labelledby=(format!("rate-{slug}-{key}"))
                                    class="flex gap-1"
                                >
                                    for n in 1..=5_u8 {
                                        choice(
                                            attrs: attributes! {
                                                name=(key)
                                                value=(n.to_string())
                                                checked=(current == Some(n))
                                                aria-label=(format!("{label} {n}"))
                                            },
                                            (n.to_string())
                                        )
                                    }
                                </div>
                            </div>
                        }
                    </div>
                </fieldset>
                if let Some(c) = category {
                    <input type="hidden" name="category" value=(c.key())>
                }
                <input type="hidden" name="sort" value=(sort.key())>
                <input type="hidden" name="q" value=(q) data-search-mirror="">
                <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
                    button(attrs: attributes! { type="submit" }, "Save vote")
                    <a
                        href=(format!("/t/{slug}"))
                        class="text-sm underline-offset-4 hover:underline"
                    >
                        "Full details"
                    </a>
                    if saved {
                        <span role="status" class="text-sm">
                            "Saved. Community scores update in a few seconds."
                        </span>
                    }
                    if let Some(note) = vote_note {
                        <span class="text-xs text-muted-foreground">(note)</span>
                    }
                </div>
            </form>
        </div>
    })
}

#[query_params(error = bad_request)]
pub(crate) struct VotePageParams {
    category: Option<String>,
    sort: Option<String>,
    q: Option<String>,
}

#[page("/t/{slug}/vote")]
pub(crate) async fn vote_page(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    let params = query_params::<VotePageParams>(cx)?;
    let category = params
        .category
        .as_deref()
        .filter(|c| !c.is_empty())
        .map(str::parse::<Category>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?;
    let sort = params
        .sort
        .as_deref()
        .map(str::parse::<Axis>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?
        .unwrap_or(Axis::Overall);
    let q = params.q.as_deref().map_or("", str::trim).to_owned();

    let pool = db(cx)?;
    let detail = tracker_by_slug(pool, slug)
        .await?
        .filter(|detail| detail.listed)
        .ok_or_not_found()?;

    let score_rows = tracker_scores(pool, detail.id).await?;
    let scores = Axis::ALL
        .map(|axis| score_rows.iter().find(|s| s.axis == axis).map(|s| s.score));
    let mine = match current_voter(cx).await? {
        Some(v) => {
            voter_tracker(pool, v, detail.id).await?.map(|m| (m.status, m.ratings))
        },
        None => None,
    };
    let signed_in = current_account(cx).await?.is_some();
    let note = vote_note(signed_in, scoring(cx)?.weights.anonymous);

    let back_href = rankings_href(category, sort, &q, None);
    let heading = format!("Vote on {}", detail.name);
    let panel = VotePanel::new(
        slug.to_owned(),
        &detail.categories,
        scores,
        mine,
        PanelContext { category, sort, q, saved: false, vote_note: note },
    );

    Ok(view! {
        <main class="mx-auto max-w-7xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">(heading)</h1>
            <p class="mt-1 mb-6 text-sm">
                <a href=(back_href) class="underline-offset-4 hover:underline">
                    "Back to rankings"
                </a>
            </p>
            vote_panel(panel: panel)
        </main>
    })
}
