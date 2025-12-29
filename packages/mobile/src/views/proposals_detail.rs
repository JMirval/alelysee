use dioxus::prelude::*;

#[component]
pub fn ProposalDetail(id: String) -> Element {
    rsx! { ui::ProposalDetailPage { id } }
}


