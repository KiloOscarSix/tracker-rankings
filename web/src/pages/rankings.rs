use std::collections::HashMap;

use rankings_core::axis::Axis;
use rankings_core::category::Category;
use rankings_core::score::AxisScore;
use rankings_core::tier::Tier;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::icon::icon;
use topcoat::icon::iconify::iconify_icon;
use topcoat::router::error::bad_request;
use topcoat::router::{page, query_params};
use topcoat::view::{View, attributes, view};

use crate::auth::sessions::current_account;
use crate::auth::voter::current_voter;
use crate::components::badge::{BadgeVariant, badge_variants};
use crate::components::input::input;
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
use crate::document::RANKINGS_SCRIPT;
use crate::pages::vote::{PanelContext, VotePanel, vote_note, vote_panel};
use crate::queries::trackers::{all_scores, listed_trackers};
use crate::queries::votes::{MyMembership, voter_memberships};
use crate::rankings::{build, matches_search, search_haystack};

#[query_params(error = bad_request)]
pub(crate) struct RankingParams {
    category: Option<String>,
    sort: Option<String>,
    q: Option<String>,
    voted: Option<String>,
}

struct RowView {
    slug: String,
    name: String,
    abbr: String,
    categories: String,
    tier: Option<Tier>,
    scores: [Option<AxisScore>; 5],
    haystack: String,
    visible: bool,
    open: bool,
    panel: Option<VotePanel>,
}

#[must_use]
pub fn rankings_href(
    category: Option<Category>,
    sort: Axis,
    q: &str,
    voted: Option<&str>,
) -> String {
    let mut params: Vec<String> = Vec::new();
    if let Some(c) = category {
        params.push(format!("category={}", c.key()));
    }
    params.push(format!("sort={}", sort.key()));
    let q = q.trim();
    if !q.is_empty() {
        params.push(format!("q={}", encode_component(q)));
    }
    if let Some(slug) = voted {
        params.push(format!("voted={}", encode_component(slug)));
    }
    format!("/?{}", params.join("&"))
}

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

fn encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            out.push(char::from(byte));
        } else {
            out.push('%');
            for nibble in [byte >> 4, byte & 0x0f] {
                if let Some(&digit) = HEX_DIGITS.get(usize::from(nibble)) {
                    out.push(char::from(digit));
                }
            }
        }
    }
    out
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
    let q = params.q.as_deref().map_or("", str::trim).to_owned();
    let voted = params.voted.as_deref();

    let pool = db(cx)?;
    let cfg = scoring(cx)?;
    let listed = build(
        listed_trackers(pool).await?,
        all_scores(pool).await?,
        filter,
        sort,
        cfg,
    );

    let voter = current_voter(cx).await?;
    let mine: HashMap<String, MyMembership> = match voter {
        Some(v) => voter_memberships(pool, v)
            .await?
            .into_iter()
            .map(|m| (m.slug.clone(), m))
            .collect(),
        None => HashMap::new(),
    };
    let signed_in = current_account(cx).await?.is_some();
    let vote_note = vote_note(signed_in, cfg.weights.anonymous);

    let intro = format!(
        "Community scores for private trackers. Trackers need an effective weight of \
         {} on Overall before they get a tier.",
        cfg.rank_min_weight
    );

    let category_links: Vec<(String, &'static str, bool)> = std::iter::once((
        rankings_href(None, sort, &q, None),
        "All",
        filter.is_none(),
    ))
    .chain(Category::ALL.into_iter().map(|c| {
        (rankings_href(Some(c), sort, &q, None), c.label(), filter == Some(c))
    }))
    .collect();
    let sort_links: Vec<(String, &'static str, Option<&'static str>, usize)> =
        Axis::ALL
            .into_iter()
            .enumerate()
            .map(|(idx, axis)| {
                (
                    rankings_href(filter, axis, &q, None),
                    axis.label(),
                    (axis == sort).then_some("descending"),
                    idx,
                )
            })
            .collect();

    let rows: Vec<RowView> = listed
        .into_iter()
        .map(|row| {
            let open = voted == Some(row.tracker.slug.as_str());
            let categories = row
                .tracker
                .categories
                .iter()
                .map(|c| c.label())
                .collect::<Vec<_>>()
                .join(", ");
            let panel = open.then(|| {
                VotePanel::new(
                    row.tracker.slug.clone(),
                    &row.tracker.categories,
                    row.scores,
                    mine.get(&row.tracker.slug).map(|m| (m.status, m.ratings)),
                    PanelContext {
                        category: filter,
                        sort,
                        q: q.clone(),
                        saved: true,
                        vote_note: vote_note.clone(),
                    },
                )
            });
            RowView {
                haystack: search_haystack(&row.tracker),
                visible: matches_search(&row.tracker, &q),
                slug: row.tracker.slug,
                name: row.tracker.name,
                abbr: row.tracker.abbr,
                categories,
                tier: row.tier,
                scores: row.scores,
                open,
                panel,
            }
        })
        .collect();
    let empty = rows.is_empty();
    let any_visible = rows.iter().any(|r| r.visible);

    Ok(view! {
        <main class="mx-auto max-w-7xl px-4 py-8">
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
                        data-keep-search=""
                    >
                        (label)
                    </a>
                }
            </nav>
            <form method="get" action="/" role="search" class="mt-4 max-w-sm">
                if let Some(c) = filter {
                    <input type="hidden" name="category" value=(c.key())>
                }
                <input type="hidden" name="sort" value=(sort.key())>
                <label for="tracker-search" class="sr-only">"Search trackers"</label>
                input(
                    attrs: attributes! {
                        id="tracker-search"
                        type="search"
                        name="q"
                        value=(q.clone())
                        placeholder="Search trackers  ( / )"
                        autocomplete="off"
                        data-search=""
                    }
                )
            </form>
            <div class="mt-6">
                if empty {
                    <p class="py-8 text-sm text-muted-foreground">
                        "No trackers in this category."
                    </p>
                } else {
                    <p
                        data-search-empty=""
                        hidden=(any_visible)
                        class="py-8 text-sm text-muted-foreground"
                    >
                        "No trackers match your search."
                    </p>
                    table(
                        attrs: attributes! {
                            data-rankings=""
                            class="max-sm:[&_td]:px-2 max-sm:[&_th]:px-2"
                        },
                        table_header(
                            table_row(
                                table_head(
                                    attrs: attributes! { class="sticky left-0 z-10 bg-background" },
                                    "Tracker"
                                )
                                table_head(
                                    attrs: attributes! { class="hidden xl:table-cell" },
                                    "Categories"
                                )
                                table_head("Tier")
                                for (href, label, aria_sort, idx) in sort_links {
                                    let head_class = if idx == 0 {
                                        "whitespace-normal!"
                                    } else {
                                        "hidden whitespace-normal! lg:table-cell"
                                    };
                                    table_head(
                                        attrs: attributes! {
                                            aria-sort=(aria_sort)
                                            class=(head_class)
                                        },
                                        <a href=(href) class="hover:underline" data-keep-search="">
                                            (label)
                                        </a>
                                    )
                                }
                                table_head(
                                    attrs: attributes! { class="w-px" },
                                    <span class="sr-only">"Vote"</span>
                                )
                            )
                        )
                        table_body(
                            for RowView {
                                slug,
                                name,
                                abbr,
                                categories,
                                tier,
                                scores,
                                haystack,
                                visible,
                                open,
                                panel,
                            } in rows {
                                let href = format!("/t/{slug}");
                                let vote_label = format!("Vote on {name}");
                                let vote_url = format!("/t/{slug}/vote");
                                let loaded_state = panel.is_some().then_some("loaded");
                                table_row(
                                    attrs: attributes! {
                                        id=(format!("tracker-{slug}"))
                                        class="group cursor-pointer data-open:bg-foreground/5"
                                        data-panel=(format!("vote-{slug}"))
                                        data-vote-url=(vote_url.clone())
                                        data-haystack=(haystack)
                                        data-open=(open.then_some(""))
                                        hidden=(!visible)
                                    },
                                    table_cell(
                                        attrs: attributes! {
                                            class="sticky left-0 z-10 bg-background whitespace-normal! group-hover:bg-[color-mix(in_oklab,var(--foreground)_5%,var(--background))] group-data-open:bg-[color-mix(in_oklab,var(--foreground)_5%,var(--background))]"
                                        },
                                        <a href=(href) class="font-medium hover:underline">
                                            (name)
                                        </a>
                                        <span class="block font-mono text-xs text-muted-foreground sm:ml-2 sm:inline">
                                            (abbr)
                                        </span>
                                    )
                                    table_cell(
                                        attrs: attributes! {
                                            class="hidden text-xs text-muted-foreground xl:table-cell whitespace-normal!"
                                        },
                                        (categories)
                                    )
                                    table_cell(tier_badge(tier: tier))
                                    for (idx, score) in scores.into_iter().enumerate() {
                                        if idx == 0 {
                                            table_cell(score_cell(score: score, compact: true))
                                        } else {
                                            table_cell(
                                                attrs: attributes! { class="hidden lg:table-cell" },
                                                score_cell(score: score, compact: true)
                                            )
                                        }
                                    }
                                    table_cell(
                                        attrs: attributes! { class="w-px text-right" },
                                        <a
                                            href=(vote_url)
                                            data-toggle=""
                                            aria-controls=(format!("vote-{slug}"))
                                            aria-expanded=(if open { "true" } else { "false" })
                                            aria-label=(vote_label)
                                            class="inline-flex size-8 items-center justify-center rounded-md text-muted-foreground hover:bg-foreground/10 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                        >
                                            icon(
                                                data: iconify_icon!("lucide:chevron-down"),
                                                attrs: attributes! {
                                                    class="size-4 transition-transform group-data-open:rotate-180"
                                                }
                                            )
                                        </a>
                                    )
                                )
                                table_row(
                                    attrs: attributes! {
                                        id=(format!("vote-{slug}"))
                                        class="hover:bg-transparent"
                                        hidden=(!(visible && open))
                                    },
                                    table_cell(
                                        attrs: attributes! { colspan="9" class="p-0! whitespace-normal!" },
                                        <div
                                            class="sticky left-0 w-[min(100%,calc(100vw-2rem))] p-4"
                                        >
                                            <div data-vote-slot="" data-state=(loaded_state)>
                                                if let Some(panel) = panel {
                                                    vote_panel(panel: panel)
                                                }
                                            </div>
                                        </div>
                                    )
                                )
                            }
                        )
                    )
                }
            </div>
            <script type="module" src=(RANKINGS_SCRIPT)></script>
        </main>
    })
}
