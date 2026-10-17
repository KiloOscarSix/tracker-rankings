use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{page, query_params};
use topcoat::view::{View, attributes, view};

use crate::auth::sessions::current_account;
use crate::components::button::button;
use crate::context::oauth_clients;

#[query_params(error = bad_request)]
pub struct LoginQuery {
    pub error: Option<String>,
}

#[page("/login")]
pub(crate) async fn login(cx: &Cx) -> Result<impl View> {
    let query = query_params::<LoginQuery>(cx)?;
    let failed = query.error.is_some();
    let signed_in = current_account(cx).await?.map(|account| account.display_name);
    let providers: Vec<(String, String)> = oauth_clients(cx)?
        .enabled()
        .map(|provider| {
            (
                format!("/auth/{}", provider.key()),
                format!("Continue with {}", provider.label()),
            )
        })
        .collect();
    let none_enabled = providers.is_empty();

    Ok(view! {
        <main class="mx-auto max-w-md px-4 py-12">
            <h1 class="text-2xl font-semibold tracking-tight">"Sign in"</h1>
            if let Some(name) = signed_in {
                <p class="mt-4 text-sm">"Signed in as " (name)</p>
            } else {
                if failed {
                    <p
                        role="alert"
                        class="mt-4 rounded-md border border-destructive/50 px-3 py-2 text-sm text-destructive"
                    >
                        "Sign-in failed. Please try again."
                    </p>
                }
                for (action, label) in providers {
                    <form method="post" action=(action) class="mt-6">
                        button(
                            attrs: attributes! { type="submit" class="w-full" },
                            (label)
                        )
                    </form>
                }
                if none_enabled {
                    <p class="mt-6 text-sm text-muted-foreground">
                        "Sign-in is not configured on this server."
                    </p>
                }
                <p class="mt-8 text-xs text-muted-foreground">
                    "Signing in sets one essential session cookie. We store your provider user id, display name and account creation date, never your email."
                </p>
            }
        </main>
    })
}
