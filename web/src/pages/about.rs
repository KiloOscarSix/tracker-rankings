use rankings_core::axis::Axis;
use rankings_core::tier::Tier;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::page;
use topcoat::view::{View, view};

use crate::context::scoring;
use crate::pages::tracker::RECRUITMENTS_WIKI;

#[page("/about")]
pub(crate) async fn about(cx: &Cx) -> Result<impl View> {
    let cfg = scoring(cx)?;

    let formula = format!(
        "Each axis score is a weighted Bayesian average, (C\u{b7}m + \u{3a3}w\u{b7}s) / \
         (C + \u{3a3}w), where C is {} (the weight of the prior), m is the mean of \
         that axis across all trackers, w is a vote's weight and s is its 1 to 5 \
         rating. Every score shows its vote count and effective weight (\u{3a3}w).",
        cfg.confidence
    );
    let account = cfg.weights.account.to_string();
    let young_account = cfg.weights.young_account.to_string();
    let anonymous = cfg.weights.anonymous.to_string();
    let young_label = format!(
        "Provider account younger than {} days",
        cfg.weights.young_account_days
    );
    let tier_intro = format!(
        "Trackers with an effective weight of at least {} on Overall are ranked by \
         percentile of their Overall score. Everything else is Unranked.",
        cfg.rank_min_weight
    );
    let tiers: Vec<(&'static str, String)> = Tier::ALL
        .into_iter()
        .zip(cfg.tier_shares)
        .map(|(tier, share)| (tier.label(), format!("{:.0}%", share * 100.0)))
        .collect();

    Ok(view! {
        <main class="mx-auto max-w-3xl px-4 py-8">
            <h1 class="text-2xl font-semibold tracking-tight">"About"</h1>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "How scores work"
                </h2>
                <p class="text-sm">
                    "Members rate trackers they are or were on, from 1 to 5, on five axes:"
                </p>
                <ul class="mt-2 list-disc space-y-1 pl-5 text-sm">
                    for axis in Axis::ALL {
                        <li>(axis.label())</li>
                    }
                </ul>
                <p class="mt-3 text-sm">(formula)</p>
            </section>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Vote weights"
                </h2>
                <dl class="grid grid-cols-[1fr_auto] gap-x-6 gap-y-1 text-sm">
                    <dt>"Signed in with Discord or Reddit"</dt>
                    <dd class="font-mono tabular-nums">(account)</dd>
                    <dt>(young_label)</dt>
                    <dd class="font-mono tabular-nums">(young_account)</dd>
                    <dt>"Anonymous"</dt>
                    <dd class="font-mono tabular-nums">(anonymous)</dd>
                </dl>
            </section>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Tiers"
                </h2>
                <p class="text-sm">(tier_intro)</p>
                <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
                    for (label, share) in tiers {
                        <dt class="font-mono">(label)</dt>
                        <dd class="font-mono tabular-nums">(share)</dd>
                    }
                </dl>
            </section>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Pairwise ranking"
                </h2>
                <p class="text-sm">
                    "Optional X-versus-Y votes are fitted with a Bradley-Terry model, using the MM algorithm from Hunter (2004)."
                </p>
            </section>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Data"
                </h2>
                <p class="text-sm">
                    "Tracker, route and invite-rank data comes from "
                    <a
                        href="https://github.com/handokota/trackerpathways"
                        class="underline"
                    >
                        "trackerpathways"
                    </a>
                    ", used under the MIT License. Official recruitments are linked from the "
                    <a href=(RECRUITMENTS_WIKI) rel="noopener" class="underline">
                        "r/TrackersInfo wiki"
                    </a>
                    " and are never scraped."
                </p>
            </section>

            <section class="mt-8">
                <h2
                    class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground"
                >
                    "Privacy"
                </h2>
                <p class="text-sm">
                    "The site sets essential cookies only: __Host-session once you sign in, oauth_state for the ten minutes a sign-in takes, and anon_id the first time you vote without an account. It stores no email addresses, IP addresses or user agents. An anonymous voter is identified only by a keyed HMAC of that cookie, your network prefix (/24 or /48) and your user agent, so it cannot be reversed."
                </p>
                <p class="mt-3 text-sm">
                    "You can review or delete everything you have voted on from "
                    <a href="/me" class="underline">"My votes"</a>
                    "."
                </p>
            </section>
        </main>
    })
}
