use rankings_core::category::Category;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::bad_request;
use topcoat::router::{page, query_params};
use topcoat::view::{View, attributes, view};

use crate::auth::voter::current_voter;
use crate::components::badge::{BadgeVariant, badge_variants};
use crate::components::button::button;
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::db;
use crate::queries::pairwise::{
    Contender,
    category_matchup,
    member_matchup,
    pairwise_ranking,
};

#[query_params(error = bad_request)]
pub(crate) struct CompareParams {
    category: Option<String>,
}

#[must_use]
pub fn compare_href(category: Option<Category>) -> String {
    category.map_or_else(
        || "/compare".to_owned(),
        |c| format!("/compare?category={}", c.key()),
    )
}

#[page("/compare")]
pub(crate) async fn compare(cx: &Cx) -> Result<impl View> {
    let params = query_params::<CompareParams>(cx)?;
    let category = params
        .category
        .as_deref()
        .map(str::parse::<Category>)
        .transpose()
        .map_err(|e| bad_request(e.to_string()))?;

    let pool = db(cx)?;
    let matchup: Vec<Contender> = match category {
        Some(c) => category_matchup(pool, c).await?,
        None => match current_voter(cx).await? {
            Some(v) => member_matchup(pool, v).await?,
            None => Vec::new(),
        },
    };
    let pair = match matchup.as_slice() {
        [a, b] => Some((a.clone(), b.clone())),
        _ => None,
    };
    let ranking = pairwise_ranking(pool).await?;

    let pool_links: Vec<(String, &'static str, bool)> =
        std::iter::once((compare_href(None), "My trackers", category.is_none()))
            .chain(
                Category::ALL.into_iter().map(|c| {
                    (compare_href(Some(c)), c.label(), category == Some(c))
                }),
            )
            .collect();
    let category_key: Option<&'static str> = category.map(Category::key);
    let skip_href = compare_href(category);
    let empty_text = if category.is_some() {
        "This category has fewer than two trackers."
    } else {
        "Declare membership on at least two trackers, or pick a category above."
    };
    let ranking_rows: Vec<(usize, String, String, String, u32)> = ranking
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            (
                i + 1,
                format!("/t/{}", row.slug),
                row.name,
                format!("{:.2}", row.strength),
                row.vote_count,
            )
        })
        .collect();
    let no_ranking = ranking_rows.is_empty();

    Ok(view! {
        <main class="mx-auto max-w-7xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">"Compare"</h1>
            <p class="mt-1 text-sm text-muted-foreground">
                "Pick the better tracker of each pair. Pairs come from the trackers you are a member of, or from one category. Votes are fitted with a Bradley–Terry model."
            </p>
            <nav aria-label="Matchup pool" class="mt-6 flex flex-wrap gap-2">
                for (href, label, active) in pool_links {
                    let variant = if active {
                        BadgeVariant::Primary
                    } else {
                        BadgeVariant::Outline
                    };
                    <a
                        href=(href)
                        class=(badge_variants(variant))
                        aria-current=(active.then_some("page"))
                    >
                        (label)
                    </a>
                }
            </nav>

            <section id="matchup" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Which is better?"
                </h2>
                if let Some((a, b)) = pair {
                    <form
                        method="post"
                        action="/vote/compare"
                        class="flex flex-wrap items-center gap-3"
                    >
                        <input type="hidden" name="a" value=(a.slug.clone())>
                        <input type="hidden" name="b" value=(b.slug.clone())>
                        if let Some(key) = category_key {
                            <input type="hidden" name="category" value=(key)>
                        }
                        button(
                            attrs: attributes! { type="submit" name="winner" value=(a.slug) },
                            (a.name)
                        )
                        <span class="text-sm text-muted-foreground">"vs"</span>
                        button(
                            attrs: attributes! { type="submit" name="winner" value=(b.slug) },
                            (b.name)
                        )
                        <a href=(skip_href) class="text-sm text-muted-foreground hover:underline">
                            "Skip"
                        </a>
                    </form>
                } else {
                    <p class="text-sm text-muted-foreground">(empty_text)</p>
                }
            </section>

            <section id="ranking" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Pairwise ranking"
                </h2>
                if no_ranking {
                    <p class="text-sm text-muted-foreground">"No pairwise votes yet."</p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("#")
                                table_head("Tracker")
                                table_head("Strength")
                                table_head("Votes")
                            )
                        )
                        table_body(
                            for (rank, href, name, strength, count) in ranking_rows {
                                table_row(
                                    table_cell((rank))
                                    table_cell(
                                        <a href=(href) class="font-medium hover:underline">
                                            (name)
                                        </a>
                                    )
                                    table_cell(
                                        <span class="font-mono tabular-nums">(strength)</span>
                                    )
                                    table_cell((count))
                                )
                            }
                        )
                    )
                }
            </section>
        </main>
    })
}
