use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{Event, expr, signal};
use topcoat::view::{View, attributes, component, view};

use super::alert_dialog::alert_dialog;
use super::button::{ButtonSize, ButtonVariant, button, button_variants};
use super::dialog::{
    dialog_content,
    dialog_description,
    dialog_footer,
    dialog_header,
    dialog_title,
};

/// A destructive action behind an alert dialog.
#[component]
pub async fn confirm_dialog(
    cx: &Cx,
    id: &'static str,
    confirming: bool,
    trigger: &'static str,
    page: String,
    title: String,
    description: String,
    action: String,
    fields: Vec<(&'static str, &'static str)>,
    submit: &'static str,
) -> Result<impl View> {
    let open = signal(cx, || confirming);
    // No fragment: browsers skip `autofocus` when the URL targets an element
    // (HTML spec, "flush autofocus candidates").
    let open_href = format!("{page}?confirm=1");
    let cancel_href = format!("{page}#{id}");
    let title_id = format!("{id}-title");
    let description_id = format!("{id}-description");
    let trigger_id = format!("{id}-trigger");
    let cancel_id = format!("{id}-cancel");

    Ok(view! {
        <a
            id=(trigger_id.clone())
            href=(open_href)
            class=(button_variants(ButtonVariant::Destructive, ButtonSize::Md))
            @click=$(|e: Event| {
                e.prevent_default();
                open.set(true);
                // Signal writes reach the DOM in a microtask; focus once the dialog shows.
                raw!("requestAnimationFrame(() => document.getElementById(${cancel_id}).focus())");
            })
        >
            (trigger)
        </a>
        alert_dialog(
            open: expr!(open.get()),
            attrs: attributes! {
                aria-labelledby=(title_id.clone())
                aria-describedby=(description_id.clone())
                aria-modal="true"
                @keydown=$(|e: Event| {
                    if e.key == "Escape" {
                        open.set(false);
                        raw!("document.getElementById(${trigger_id}).focus()");
                    } else if e.key == "Tab" {
                        e.prevent_default();
                        if e.shift_key {
                            raw!("(() => { const f = [...document.activeElement.closest('dialog').querySelectorAll('a[href], button:not([disabled])')]; const i = f.indexOf(document.activeElement); f[(i < 1 ? f.length : i) - 1].focus(); })()");
                        } else {
                            raw!("(() => { const f = [...document.activeElement.closest('dialog').querySelectorAll('a[href], button:not([disabled])')]; f[(f.indexOf(document.activeElement) + 1) % f.length].focus(); })()");
                        }
                    }
                })
            },
            dialog_content(
                dialog_header(
                    dialog_title(attrs: attributes! { id=(title_id) }, (title))
                    dialog_description(
                        attrs: attributes! { id=(description_id) },
                        (description)
                    )
                )
                dialog_footer(
                    <a
                        id=(cancel_id.clone())
                        href=(cancel_href)
                        class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
                        autofocus=(confirming)
                        @click=$(|e: Event| {
                            e.prevent_default();
                            open.set(false);
                            raw!("document.getElementById(${trigger_id}).focus()");
                        })
                    >
                        "Cancel"
                    </a>
                    <form method="post" action=(action)>
                        for (name, value) in fields {
                            <input type="hidden" name=(name) value=(value)>
                        }
                        button(
                            variant: ButtonVariant::Destructive,
                            attrs: attributes! { type="submit" },
                            (submit)
                        )
                    </form>
                )
            )
        )
    })
}
