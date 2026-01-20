use dioxus::prelude::*;
use api::types::{ContentTargetType, Video};

const VIDEO_FEED_CSS: Asset = asset!("/assets/styling/video_feed.css");

#[component]
pub fn VideoFeed(
    starting_video_id: Option<String>,
    filter_target_type: Option<ContentTargetType>,
    filter_target_id: Option<String>,
) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    // State management
    let mut current_index = use_signal(|| 0usize);
    let mut videos = use_signal(|| Vec::<Video>::new());
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    // Load initial videos
    let filter_context = (filter_target_type, filter_target_id.clone());
    use_effect(move || {
        let token = token.clone();
        let filter = filter_context.clone();
        spawn(async move {
            loading.set(true);

            let result = if let (Some(target_type), Some(target_id)) = filter {
                // Single content mode
                api::list_single_content_videos(target_type, target_id, 5, 0).await
            } else {
                // Discovery mode
                api::list_feed_videos(token, 5, 0).await
            };

            match result {
                Ok(vids) => {
                    videos.set(vids);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: VIDEO_FEED_CSS }

        div { class: "video-feed-container",
            if loading() {
                p { "Loading videos..." }
            } else if let Some(err) = error_msg() {
                p { class: "error", "Error: {err}" }
            } else if videos().is_empty() {
                p { "No videos available" }
            } else {
                div { class: "video-feed-scroll",
                    for (idx, video) in videos().iter().enumerate() {
                        div {
                            key: "{video.id}",
                            class: "video-feed-item",
                            "Video {idx}: {video.id}"
                        }
                    }
                }
            }
        }
    }
}
