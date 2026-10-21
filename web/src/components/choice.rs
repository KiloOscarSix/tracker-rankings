use topcoat::Result;
use topcoat::view::{Attributes, Child, StaticClass, View, class, component, view};

const CHOICE: StaticClass = class!(
    "inline-flex h-8 min-w-8 cursor-pointer select-none items-center justify-center \
     rounded-md border border-border px-2.5 text-sm tabular-nums transition-colors \
     hover:bg-foreground/5 has-checked:border-primary has-checked:bg-primary \
     has-checked:text-primary-foreground has-focus-visible:ring-2 \
     has-focus-visible:ring-ring has-focus-visible:ring-offset-2 \
     has-focus-visible:ring-offset-background",
);

#[component]
pub async fn choice(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <label class=(class!(CHOICE, attrs.remove("class")))>
            <input type="radio" class="sr-only" (attrs)>
            (child)
        </label>
    })
}
