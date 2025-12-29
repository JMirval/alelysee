use dioxus::prelude::*;

const HERO_CSS: Asset = asset!("/assets/styling/hero.css");

#[component]
pub fn Hero() -> Element {
    let lang = crate::use_lang()();
    rsx! {
        document::Link { rel: "stylesheet", href: HERO_CSS }

        div {
            id: "hero",
            div { id: "links",
                h1 { {crate::t(lang, "app.name")} }
                p { {crate::t(lang, "home.subtitle")} }

                div { class: "cta_row",
                    a { class: "btn primary", href: "/proposals", {crate::t(lang, "home.cta.proposals")} }
                    a { class: "btn", href: "/programs", {crate::t(lang, "home.cta.programs")} }
                }
                p { class: "hint", {crate::t(lang, "home.tip")} }
            }
        }
    }
}
