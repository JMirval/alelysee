use dioxus::prelude::*;

const THEME_CSS: Asset = asset!("/assets/styling/theme.css");

#[component]
pub fn CivicTheme() -> Element {
    rsx! { document::Link { rel: "stylesheet", href: THEME_CSS } }
}
