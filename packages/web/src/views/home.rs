use dioxus::prelude::*;
use ui::Hero;

#[component]
pub fn Home() -> Element {
    rsx! {
        Hero {}
        div { class: "panel",
            h2 { "Start here" }
            p { class: "hint", "Explore what people propose, bundle proposals into programs, and discuss with votes, comments, and videos." }
            div { class: "cta_row",
                a { class: "btn primary", href: "/proposals", "Go to proposals" }
                a { class: "btn", href: "/programs", "Go to programs" }
                a { class: "btn", href: "/me", "Your profile" }
            }
        }
    }
}
