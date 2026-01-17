use dioxus::prelude::*;

#[component]
pub fn AuthResetPassword() -> Element {
    rsx! { ui::RequestPasswordResetForm {} }
}
