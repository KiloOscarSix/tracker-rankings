use ladder_core::axis::Axis;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::RouterErrorExt;
use topcoat::router::{page, path_param};
use topcoat::view::{View, attributes, view};

use crate::components::score::score_cell;
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::db;
use crate::queries::trackers::{
    incoming_routes,
    outgoing_routes,
    tracker_by_slug,
    tracker_scores,
};

pub const RECRUITMENTS_WIKI: &str =
    "https://www.reddit.com/r/TrackersInfo/wiki/official_recruitments/";

path_param!(pub slug);

#[page("/t/{slug}")]
pub(crate) async fn tracker(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    let pool = db(cx)?;
    let detail = tracker_by_slug(pool, slug).await?.ok_or_not_found()?;
    let scores = tracker_scores(pool, detail.id).await?;
    let outgoing = outgoing_routes(pool, detail.id).await?;
    let incoming = incoming_routes(pool, detail.id).await?;

    let mut facts: Vec<String> = Vec::new();
    if !detail.categories.is_empty() {
        let labels: Vec<&str> =
            detail.categories.iter().map(|c| c.label()).collect();
        facts.push(labels.join(", "));
    }
    if let Some(country) = detail.country.as_deref().filter(|c| !c.is_empty()) {
        facts.push(country.to_owned());
    }
    if let Some(year) = detail.founded_year {
        facts.push(format!("Founded {year}"));
    }
    let facts = facts.join(" · ");

    let unlock_days = detail.unlock_days.map(|days| format!("About {days} days"));
    let unlock_reqs: Vec<String> =
        detail.unlock_reqs.as_deref().map_or_else(Vec::new, |reqs| {
            reqs.split(';')
                .map(str::trim)
                .filter(|req| !req.is_empty())
                .map(str::to_owned)
                .collect()
        });
    let has_unlock = unlock_days.is_some() || !unlock_reqs.is_empty();

    let axis_scores: Vec<_> = Axis::ALL
        .into_iter()
        .map(|axis| {
            (axis.label(), scores.iter().find(|s| s.axis == axis).map(|s| s.score))
        })
        .collect();
    let no_scores = scores.is_empty();

    let name = detail.name;
    let abbr = detail.abbr;
    let listed = detail.listed;
    let description = detail.description;
    let has_facts = !facts.is_empty();

    Ok(view! {
        <main class="mx-auto max-w-6xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">
                (name)
                if let Some(abbr) = abbr {
                    <span class="ml-2 font-mono text-base text-muted-foreground">
                        (abbr)
                    </span>
                }
            </h1>
            if has_facts {
                <p class="mt-1 text-sm text-muted-foreground">(facts)</p>
            }
            if let Some(description) = description {
                <p class="mt-4 max-w-3xl text-sm">(description)</p>
            }
            if !listed {
                <p class="mt-4 text-sm text-muted-foreground">
                    "trackerpathways lists invite routes for this tracker but does not describe it."
                </p>
            }

            <section id="scores" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Community scores"
                </h2>
                if no_scores {
                    <p class="text-sm text-muted-foreground">"No votes yet."</p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("Axis")
                                table_head("Score")
                            )
                        )
                        table_body(
                            for (label, score) in axis_scores {
                                table_row(
                                    table_cell((label))
                                    table_cell(score_cell(score: score))
                                )
                            }
                        )
                    )
                }
            </section>

            <section id="invite-rank" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Reaching invite rank"
                </h2>
                if has_unlock {
                    if let Some(days) = unlock_days {
                        <p class="text-sm">(days)</p>
                    }
                    if !unlock_reqs.is_empty() {
                        <ul class="mt-2 list-disc space-y-1 pl-5 text-sm">
                            for req in unlock_reqs {
                                <li>(req)</li>
                            }
                        </ul>
                    }
                } else {
                    <p class="text-sm text-muted-foreground">
                        "No invite-rank requirements recorded."
                    </p>
                }
            </section>

            <section id="next-steps" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Next steps from here"
                </h2>
                if outgoing.is_empty() {
                    <p class="text-sm text-muted-foreground">
                        "No outgoing invite routes."
                    </p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("Tracker")
                                table_head("Community score")
                                table_head("Wait")
                                table_head("Requirements")
                                table_head("Open")
                            )
                        )
                        table_body(
                            for route in outgoing {
                                let href = format!("/t/{}", route.slug);
                                let wait = wait_label(route.days);
                                table_row(
                                    table_cell(
                                        <a href=(href) class="font-medium hover:underline">
                                            (route.name)
                                        </a>
                                    )
                                    table_cell(score_cell(score: route.overall))
                                    table_cell((wait))
                                    table_cell(
                                        attrs: attributes! { class="whitespace-normal!" },
                                        (route.reqs)
                                    )
                                    table_cell(
                                        attrs: attributes! { class="whitespace-normal!" },
                                        (route.active)
                                    )
                                )
                            }
                        )
                    )
                }
            </section>

            <section id="incoming" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Invited from"
                </h2>
                if incoming.is_empty() {
                    <p class="text-sm text-muted-foreground">
                        "No incoming invite routes."
                    </p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("Tracker")
                                table_head("Wait")
                                table_head("Requirements")
                                table_head("Open")
                            )
                        )
                        table_body(
                            for route in incoming {
                                let href = format!("/t/{}", route.slug);
                                let wait = wait_label(route.days);
                                table_row(
                                    table_cell(
                                        <a href=(href) class="font-medium hover:underline">
                                            (route.name)
                                        </a>
                                    )
                                    table_cell((wait))
                                    table_cell(
                                        attrs: attributes! { class="whitespace-normal!" },
                                        (route.reqs)
                                    )
                                    table_cell(
                                        attrs: attributes! { class="whitespace-normal!" },
                                        (route.active)
                                    )
                                )
                            }
                        )
                    )
                }
            </section>

            <p class="mt-8 text-sm">
                <a href=(RECRUITMENTS_WIKI) rel="noopener" class="underline">
                    "Official recruitments (r/TrackersInfo wiki)"
                </a>
            </p>
        </main>
    })
}

fn wait_label(days: Option<i32>) -> String {
    match days {
        Some(0) => "No wait".to_owned(),
        Some(days) => format!("{days} days"),
        None => "Unknown".to_owned(),
    }
}
