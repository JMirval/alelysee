use dioxus::prelude::*;

#[component]
pub fn Videos() -> Element {
    rsx! {
        ui::VideoFeed {
            starting_video_id: None,
            filter_target_type: None,
            filter_target_id: None,
        }
    }
}

#[component]
pub fn VideoDetail(id: String) -> Element {
    rsx! {
        ui::VideoFeed {
            starting_video_id: Some(id),
            filter_target_type: None,
            filter_target_id: None,
        }
    }
}
