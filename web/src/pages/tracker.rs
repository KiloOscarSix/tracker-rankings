use rankings_core::axis::Axis;
use rankings_core::membership::MembershipStatus;
use rankings_core::score::Score;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::RouterErrorExt;
use topcoat::router::{page, path_param, query_params};
use topcoat::view::{View, attributes, view};

use crate::auth::sessions::current_account;
use crate::auth::voter::current_voter;
use crate::components::button::button;
use crate::components::confirm::confirm_dialog;
use crate::components::label::label;
use crate::components::radio_group::{radio_group, radio_group_item};
use crate::components::score::score_cell;
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::{db, scoring};
use crate::queries::trackers::{
    incoming_routes,
    outgoing_routes,
    tracker_by_slug,
    tracker_scores,
};
use crate::queries::votes::voter_tracker;
use crate::rankings::axis_index;
use crate::votes::{CONFIRMED, REMOVE};

pub const RECRUITMENTS_WIKI: &str =
    "https://www.reddit.com/r/TrackersInfo/wiki/official_recruitments/";

path_param!(pub slug);

#[query_params(error = bad_request)]
pub struct TrackerQuery {
    pub confirm: Option<String>,
}

#[page("/t/{slug}")]
pub(crate) async fn tracker(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    let confirming = query_params::<TrackerQuery>(cx)?.confirm.is_some();
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

    let voter = current_voter(cx).await?;
    let mine = match voter {
        Some(v) if detail.listed => voter_tracker(pool, v, detail.id).await?,
        _ => None,
    };
    let signed_in = current_account(cx).await?.is_some();
    let anonymous_weight = scoring(cx)?.weights.anonymous;

    let membership_action = format!("/vote/{slug}/membership");
    let leave_action = membership_action.clone();
    let tracker_href = format!("/t/{slug}");
    let leave_title = format!("Remove your {} membership?", detail.name);
    let leave_description =
        format!("Your ratings for {} are deleted with it.", detail.name);
    let ratings_action = format!("/vote/{slug}/ratings");
    let my_status = mine.as_ref().map(|m| m.status);
    let is_member = my_status.is_some();
    let status_options: Vec<(&'static str, &'static str, bool)> =
        MembershipStatus::ALL
            .into_iter()
            .map(|status| (status.key(), status.label(), my_status == Some(status)))
            .collect();
    let rating_rows: Vec<(&'static str, &'static str, Option<u8>)> =
        mine.as_ref().map_or_else(Vec::new, |m| {
            Axis::ALL
                .into_iter()
                .map(|axis| {
                    let current = m
                        .ratings
                        .get(axis_index(axis))
                        .copied()
                        .flatten()
                        .map(Score::get);
                    (axis.key(), axis.label(), current)
                })
                .collect()
        });
    let vote_note = if signed_in {
        "Community scores update a few seconds after each vote.".to_owned()
    } else {
        format!(
            "Anonymous votes count {anonymous_weight}\u{d7}; sign in for full weight. \
             Community scores update a few seconds after each vote."
        )
    };

    let name = detail.name;
    let abbr = detail.abbr;
    let listed = detail.listed;
    let description = detail.description;
    let has_facts = !facts.is_empty();

    Ok(view! {
        <main class="mx-auto max-w-7xl px-4 py-8">
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
                            for (axis_label, score) in axis_scores {
                                table_row(
                                    table_cell((axis_label))
                                    table_cell(score_cell(score: score))
                                )
                            }
                        )
                    )
                }
            </section>

            if listed {
                <section id="vote" class="mt-8">
                    <h2
                        class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                    >
                        "Your vote"
                    </h2>
                    <form
                        method="post"
                        action=(membership_action)
                        class="flex flex-wrap items-center gap-4"
                    >
                        radio_group(
                            attrs: attributes! {
                                aria-label="Membership"
                                class="sm:grid-flow-col sm:auto-cols-max sm:gap-x-4"
                            },
                            for (key, text, checked) in status_options {
                                <div class="flex items-center gap-2">
                                    radio_group_item(
                                        attrs: attributes! {
                                            id=(format!("membership-{key}"))
                                            name="status"
                                            value=(key)
                                            checked=(checked)
                                        }
                                    )
                                    label(
                                        attrs: attributes! { for=(format!("membership-{key}")) },
                                        (text)
                                    )
                                </div>
                            }
                        )
                        button(attrs: attributes! { type="submit" }, "Save membership")
                    </form>
                    if is_member {
                        <div id="leave" class="mt-4">
                            confirm_dialog(
                                id: "leave",
                                confirming: confirming,
                                trigger: "Not a member (removes my ratings)",
                                page: tracker_href,
                                title: leave_title,
                                description: leave_description,
                                action: leave_action,
                                fields: vec![("status", REMOVE), ("confirm", CONFIRMED)],
                                submit: "Remove membership"
                            )
                        </div>
                    }
                    if is_member {
                        <form method="post" action=(ratings_action) class="mt-4">
                            for (key, text, current) in rating_rows {
                                <div class="flex flex-wrap items-center gap-4 py-1">
                                    <span id=(format!("axis-{key}")) class="w-48 text-sm">
                                        (text)
                                    </span>
                                    radio_group(
                                        attrs: attributes! {
                                            aria-labelledby=(format!("axis-{key}"))
                                            class="grid-flow-col auto-cols-max gap-x-4"
                                        },
                                        for n in 1..=5_u8 {
                                            <div class="flex items-center gap-1.5">
                                                radio_group_item(
                                                    attrs: attributes! {
                                                        id=(format!("rate-{key}-{n}"))
                                                        name=(key)
                                                        value=(n.to_string())
                                                        checked=(current == Some(n))
                                                    }
                                                )
                                                label(
                                                    attrs: attributes! { for=(format!("rate-{key}-{n}")) },
                                                    (n.to_string())
                                                )
                                            </div>
                                        }
                                    )
                                </div>
                            }
                            button(
                                attrs: attributes! { type="submit" class="mt-3" },
                                "Save ratings"
                            )
                        </form>
                    } else {
                        <p class="mt-4 text-sm text-muted-foreground">
                            "Say whether you are or were a member to rate this tracker."
                        </p>
                    }
                    <p class="mt-3 text-xs text-muted-foreground">(vote_note)</p>
                </section>
            }

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
