use rankings_core::axis::Axis;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::{SeeOther, bad_request, see_other};
use topcoat::router::{page, query_params, route};
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, view};

use crate::auth::anon::{clear_anon_cookie, existing_anon_voter};
use crate::auth::sessions::{current_account, sign_out};
use crate::auth::voter::current_voter;
use crate::components::confirm::confirm_dialog;
use crate::components::table::{
    table,
    table_body,
    table_cell,
    table_head,
    table_header,
    table_row,
};
use crate::context::{db, score_signal};
use crate::queries::votes::{
    delete_account,
    delete_anon_identity,
    voter_memberships,
};
use crate::rankings::axis_index;
use crate::votes::{CONFIRMED, ConfirmForm, confirmed};

#[query_params(error = bad_request)]
pub struct MeQuery {
    pub deleted: Option<String>,
    pub confirm: Option<String>,
}

#[page]
pub(crate) async fn me(cx: &Cx) -> Result<impl View> {
    let query = query_params::<MeQuery>(cx)?;
    let deleted = query.deleted.is_some();
    let confirming = query.confirm.is_some();
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
    let delete_description = "Your memberships, ratings and pairwise votes are \
                              deleted, and a signed-in account is deleted and \
                              signed out. This cannot be undone."
        .to_owned();
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
        <main class="mx-auto max-w-7xl px-4 py-8">
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
                                        <a
                                            class="font-medium hover:underline"
                                            (link_attrs(cx, href, prefetch_mode(cx)))
                                        >
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
                    <div class="mt-3">
                        confirm_dialog(
                            id: "delete",
                            confirming: confirming,
                            trigger: "Delete my data",
                            page: "/me".to_owned(),
                            title: "Delete all your data?".to_owned(),
                            description: delete_description,
                            action: "/me/delete".to_owned(),
                            fields: vec![("confirm", CONFIRMED)],
                            submit: "Delete my data"
                        )
                    </div>
                </section>
            }
        </main>
    })
}

#[route(POST "./delete")]
pub(crate) async fn delete_my_data(
    cx: &Cx,
    Form(form): Form<ConfirmForm>,
) -> Result<SeeOther> {
    if !confirmed(form.confirm.as_deref()) {
        return Err(bad_request("confirm deleting your data").into());
    }
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
