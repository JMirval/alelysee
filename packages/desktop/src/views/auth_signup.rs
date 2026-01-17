use dioxus::prelude::*;

#[component]
pub fn AuthSignUp() -> Element {
    rsx! { ui::SignUpForm {} }
}
