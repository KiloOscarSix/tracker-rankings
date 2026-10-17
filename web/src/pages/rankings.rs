use rankings_core::axis::Axis;
use rankings_core::category::Category;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::bad_request;
use topcoat::router::{page, query_params};
use topcoat::view::{View, attributes, view};

use crate::components::badge::{BadgeVariant, badge_variants};
use crate::components::score::{score_cell, tier_badge};
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::{db, scoring};
use crate::queries::trackers::{all_scores, listed_trackers};
use crate::rankings::build;

#[query_params(error = bad_request)]
pub(crate) struct RankingParams {
    category: Option<String>,
    sort: Option<String>,
}

#[page("/")]
pub(crate) async fn rankings(cx: &Cx) -> Result<impl View> {
    let params = query_params::<RankingParams>(cx)?;
    let filter = params
        .category
        .as_deref()
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

    let pool = db(cx)?;
    let cfg = scoring(cx)?;
    let rows = build(
        listed_trackers(pool).await?,
        all_scores(pool).await?,
        filter,
        sort,
        cfg,
    );

    let intro = format!(
        "Community scores for private trackers. Trackers need an effective weight of \
         {} on Overall before they get a tier.",
        cfg.rank_min_weight
    );

    let category_prefix =
        filter.map_or_else(String::new, |c| format!("category={}&", c.key()));
    let category_links: Vec<(String, &'static str, bool)> =
        std::iter::once((format!("/?sort={}", sort.key()), "All", filter.is_none()))
            .chain(Category::ALL.into_iter().map(|c| {
                (
                    format!("/?category={}&sort={}", c.key(), sort.key()),
                    c.label(),
                    filter == Some(c),
                )
            }))
            .collect();
    let sort_links: Vec<(String, &'static str, Option<&'static str>)> = Axis::ALL
        .into_iter()
        .map(|axis| {
            (
                format!("/?{category_prefix}sort={}", axis.key()),
                axis.label(),
                (axis == sort).then_some("descending"),
            )
        })
        .collect();
    let empty = rows.is_empty();

    Ok(view! {
        <main class="mx-auto max-w-6xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">"Rankings"</h1>
            <p class="mt-1 text-sm text-muted-foreground">(intro)</p>
            <nav aria-label="Category" class="mt-6 flex flex-wrap gap-2">
                for (href, label, active) in category_links {
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
            <div class="mt-6">
                if empty {
                    <p class="py-8 text-sm text-muted-foreground">
                        "No trackers in this category."
                    </p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("Tracker")
                                table_head("Categories")
                                table_head("Tier")
                                for (href, label, aria_sort) in sort_links {
                                    table_head(
                                        attrs: attributes! { aria-sort=(aria_sort) },
                                        <a href=(href) class="hover:underline">(label)</a>
                                    )
                                }
                            )
                        )
                        table_body(
                            for row in rows {
                                let href = format!("/t/{}", row.tracker.slug);
                                let categories = row
                                    .tracker
                                    .categories
                                    .iter()
                                    .map(|c| c.label())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                table_row(
                                    table_cell(
                                        <a href=(href) class="font-medium hover:underline">
                                            (row.tracker.name)
                                        </a>
                                        <span class="ml-2 font-mono text-xs text-muted-foreground">
                                            (row.tracker.abbr)
                                        </span>
                                    )
                                    table_cell(
                                        attrs: attributes! { class="text-xs text-muted-foreground" },
                                        (categories)
                                    )
                                    table_cell(tier_badge(tier: row.tier))
                                    for score in row.scores {
                                        table_cell(score_cell(score: score))
                                    }
                                )
                            }
                        )
                    )
                }
            </div>
        </main>
    })
}
