use rankings_core::axis::Axis;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{page, query_params};
use topcoat::view::{View, attributes, view};

use crate::auth::sessions::current_account;
use crate::auth::voter::current_voter;
use crate::components::button::{ButtonVariant, button};
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::db;
use crate::queries::votes::voter_memberships;
use crate::rankings::axis_index;

#[query_params(error = bad_request)]
pub struct MeQuery {
    pub deleted: Option<String>,
}

#[page("/me")]
pub(crate) async fn me(cx: &Cx) -> Result<impl View> {
    let query = query_params::<MeQuery>(cx)?;
    let deleted = query.deleted.is_some();
    let account = current_account(cx).await?;
    let voter = current_voter(cx).await?;
    let memberships = match voter {
        Some(v) => voter_memberships(db(cx)?, v).await?,
        None => Vec::new(),
    };

    let identity = account.as_ref().map_or_else(
        || {
            "You are voting anonymously. Anonymous votes are tied to this browser's \
             cookie and your network."
                .to_owned()
        },
        |account| {
            format!(
                "Signed in as {} with {}.",
                account.display_name,
                account.provider.label()
            )
        },
    );
    let can_delete = account.is_some() || voter.is_some();
    let no_memberships = memberships.is_empty();
    let rows: Vec<(String, String, &'static str, Vec<String>)> = memberships
        .into_iter()
        .map(|membership| {
            let scores = Axis::ALL
                .into_iter()
                .map(|axis| {
                    membership
                        .ratings
                        .get(axis_index(axis))
                        .copied()
                        .flatten()
                        .map_or_else(
                            || "\u{2014}".to_owned(),
                            |s| s.get().to_string(),
                        )
                })
                .collect();
            (
                format!("/t/{}", membership.slug),
                membership.name,
                membership.status.label(),
                scores,
            )
        })
        .collect();

    Ok(view! {
        <main class="mx-auto max-w-6xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">"My votes"</h1>
            if deleted {
                <p
                    role="alert"
                    class="mt-4 rounded-md border border-border px-3 py-2 text-sm"
                >
                    "Your data was deleted."
                </p>
            }
            <p class="mt-4 text-sm text-muted-foreground">(identity)</p>

            <section id="memberships" class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Memberships and ratings"
                </h2>
                if no_memberships {
                    <p class="text-sm text-muted-foreground">
                        "You have not declared any memberships yet."
                    </p>
                } else {
                    table(
                        table_header(
                            table_row(
                                table_head("Tracker")
                                table_head("Status")
                                for axis in Axis::ALL {
                                    table_head((axis.label()))
                                }
                            )
                        )
                        table_body(
                            for (href, name, status, scores) in rows {
                                table_row(
                                    table_cell(
                                        <a href=(href) class="font-medium hover:underline">
                                            (name)
                                        </a>
                                    )
                                    table_cell((status))
                                    for score in scores {
                                        table_cell((score))
                                    }
                                )
                            }
                        )
                    )
                }
            </section>

            if can_delete {
                <section id="delete" class="mt-8">
                    <h2
                        class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                    >
                        "Delete my data"
                    </h2>
                    <p class="text-sm">
                        "Deletes your memberships, ratings and pairwise votes. A signed-in account is deleted and signed out."
                    </p>
                    <form method="post" action="/me/delete" class="mt-3">
                        button(
                            variant: ButtonVariant::Destructive,
                            attrs: attributes! { type="submit" },
                            "Delete my data"
                        )
                    </form>
                </section>
            }
        </main>
    })
}
