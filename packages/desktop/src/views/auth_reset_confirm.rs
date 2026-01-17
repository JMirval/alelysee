use dioxus::prelude::*;

#[component]
pub fn AuthResetConfirm() -> Element {
    rsx! { ui::ResetPasswordConfirmForm {} }
}
