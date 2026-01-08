use dioxus::prelude::*;

#[component]
pub fn ProgramDetail(id: String) -> Element {
    rsx! { ui::ProgramDetailPage { id } }
}
