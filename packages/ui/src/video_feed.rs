use dioxus::prelude::*;
use api::types::{ContentTargetType, Video};

#[component]
pub fn VideoFeed(
    starting_video_id: Option<String>,
    filter_target_type: Option<ContentTargetType>,
    filter_target_id: Option<String>,
) -> Element {
    rsx! {
        div { class: "video-feed-placeholder",
            p { "VideoFeed component - coming soon" }
        }
    }
}
