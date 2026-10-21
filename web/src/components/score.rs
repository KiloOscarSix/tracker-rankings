use rankings_core::score::AxisScore;
use rankings_core::tier::Tier;
use topcoat::Result;
use topcoat::view::{View, attributes, component, view};

use super::badge::{BadgeVariant, badge};

#[component]
pub async fn tier_badge(tier: Option<Tier>) -> Result<impl View> {
    Ok(view! {
        if let Some(t) = tier {
            badge(
                variant: BadgeVariant::Primary,
                attrs: attributes! { class="rounded-sm font-mono" },
                (t.label())
            )
        } else {
            badge(variant: BadgeVariant::Outline, "Unranked")
        }
    })
}

#[component]
pub async fn score_cell(
    score: Option<AxisScore>,
    #[default] compact: bool,
) -> Result<impl View> {
    let meta = if compact {
        "block text-xs text-muted-foreground max-sm:hidden"
    } else {
        "block text-xs text-muted-foreground"
    };

    Ok(view! {
        if let Some(s) = score {
            <span class="font-mono tabular-nums">(format!("{:.2}", s.score))</span>
            <span class=(meta)>
                (format!("{} votes · {:.1} weight", s.vote_count, s.effective_weight))
            </span>
        } else {
            <span class="text-muted-foreground">"—"</span>
        }
    })
}
