use dioxus::prelude::*;

#[component]
pub fn AuthVerify(token: Option<String>) -> Element {
    rsx! { ui::VerifyEmailPage { token } }
}
