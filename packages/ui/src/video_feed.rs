use dioxus::prelude::*;
use api::types::{ContentTargetType, Video};

const VIDEO_FEED_CSS: Asset = asset!("/assets/styling/video_feed.css");

#[component]
fn VideoOverlay(video_id: String, initial_vote_score: i64) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    let mut vote_score = use_signal(|| initial_vote_score);
    let mut user_vote = use_signal(|| 0i16); // -1, 0, or 1
    let mut is_bookmarked = use_signal(|| false);
    let comment_count = use_signal(|| 0i32);

    // Clone for each closure
    let token_upvote = token.clone();
    let video_id_upvote = video_id.clone();
    let token_downvote = token.clone();
    let video_id_downvote = video_id.clone();
    let token_bookmark = token.clone();
    let video_id_bookmark = video_id.clone();

    let on_upvote = move |_| {
        let token = token_upvote.clone();
        let vid = video_id_upvote.clone();
        let current_vote = user_vote();

        spawn(async move {
            let new_vote = if current_vote == 1 { 0 } else { 1 };
            match api::set_vote(token, ContentTargetType::Video, vid, new_vote).await {
                Ok(_state) => {
                    user_vote.set(new_vote);
                    // Update score optimistically
                    let score_change = new_vote as i64 - current_vote as i64;
                    vote_score.set(vote_score() + score_change);
                }
                Err(_e) => {
                    // Failed to upvote
                }
            }
        });
    };

    let on_downvote = move |_| {
        let token = token_downvote.clone();
        let vid = video_id_downvote.clone();
        let current_vote = user_vote();

        spawn(async move {
            let new_vote = if current_vote == -1 { 0 } else { -1 };
            match api::set_vote(token, ContentTargetType::Video, vid, new_vote).await {
                Ok(_state) => {
                    user_vote.set(new_vote);
                    // Update score optimistically
                    let score_change = new_vote as i64 - current_vote as i64;
                    vote_score.set(vote_score() + score_change);
                }
                Err(_e) => {
                    // Failed to downvote
                }
            }
        });
    };

    let on_bookmark = move |_| {
        let token = token_bookmark.clone();
        let vid = video_id_bookmark.clone();

        spawn(async move {
            match api::bookmark_video(token, vid).await {
                Ok(bookmarked) => {
                    is_bookmarked.set(bookmarked);
                }
                Err(_e) => {
                    // Failed to bookmark
                }
            }
        });
    };

    rsx! {
        div { class: "video-overlay",
            // Upvote button
            button {
                class: if user_vote() == 1 { "overlay-btn active" } else { "overlay-btn" },
                onclick: on_upvote,
                div { class: "btn-icon", "▲" }
                div { class: "btn-count", "{vote_score()}" }
            }

            // Downvote button
            button {
                class: if user_vote() == -1 { "overlay-btn active" } else { "overlay-btn" },
                onclick: on_downvote,
                div { class: "btn-icon", "▼" }
            }

            // Bookmark button
            button {
                class: if is_bookmarked() { "overlay-btn active" } else { "overlay-btn" },
                onclick: on_bookmark,
                div { class: "btn-icon", if is_bookmarked() { "🔖" } else { "🔖" } }
            }

            // Comment button (TODO: open panel)
            button {
                class: "overlay-btn",
                div { class: "btn-icon", "💬" }
                div { class: "btn-count", "{comment_count()}" }
            }
        }
    }
}

#[component]
fn VideoMetadata(video: Video) -> Element {
    // Load proposal/program info
    let mut content_title = use_signal(|| String::from("Loading..."));
    let author_name = use_signal(|| String::from(""));

    let target_id = video.target_id.to_string();
    use_effect(move || {
        let target_type = video.target_type;
        let tid = target_id.clone();

        spawn(async move {
            match target_type {
                ContentTargetType::Proposal => {
                    if let Ok(proposal) = api::get_proposal(tid).await {
                        content_title.set(proposal.title);
                        // TODO: Load author name from proposal.author_user_id
                    }
                }
                ContentTargetType::Program => {
                    if let Ok(program_detail) = api::get_program(tid).await {
                        content_title.set(program_detail.program.title);
                        // TODO: Load author name from program_detail.program.author_user_id
                    }
                }
                _ => {}
            }
        });
    });

    rsx! {
        div { class: "video-metadata",
            h3 { class: "metadata-title", "{content_title()}" }
            p { class: "metadata-author", "By {author_name()}" }
            a {
                class: "metadata-link",
                href: match video.target_type {
                    ContentTargetType::Proposal => format!("/proposals/{}", video.target_id),
                    ContentTargetType::Program => format!("/programs/{}", video.target_id),
                    _ => "#".to_string(),
                },
                {match video.target_type {
                    ContentTargetType::Proposal => "View full proposal",
                    ContentTargetType::Program => "View full program",
                    _ => "View content",
                }}
            }
        }
    }
}

#[component]
fn VideoFeedItem(video: Video, is_active: bool) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let cfg = use_resource(|| async move { api::public_config().await });

    let mut view_tracked = use_signal(|| false);

    // Track view after 2 seconds of being active
    use_effect(move || {
        if is_active && !view_tracked() {
            let token = token.clone();
            let video_id = video.id.to_string();
            spawn(async move {
                // Wait 2 seconds
                gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;

                // Mark as viewed
                let _ = api::mark_video_viewed(token, video_id).await;
            });
            view_tracked.set(true);
        }
    });

    rsx! {
        div { class: "video-feed-item",
            match cfg() {
                None => rsx! { p { class: "hint", "Loading player..." } },
                Some(Err(_)) => rsx! { p { class: "hint", "Player not configured." } },
                Some(Ok(cfg)) => {
                    let src = cfg.media_base_url.as_ref().map(|base| {
                        format!("{}/{}", base.trim_end_matches('/'), video.storage_key)
                    });

                    rsx! {
                        if let Some(src) = src {
                            video {
                                class: "video-feed-player",
                                src: "{src}",
                                muted: false,
                                autoplay: is_active,
                                playsinline: true,
                                preload: "auto",
                            }
                        } else {
                            p { class: "hint", "Set MEDIA_BASE_URL to enable playback." }
                        }
                    }
                }
            }

            VideoOverlay {
                video_id: video.id.to_string(),
                initial_vote_score: video.vote_score,
            }

            VideoMetadata {
                video: video.clone(),
            }
        }
    }
}

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
                        VideoFeedItem {
                            key: "{video.id}",
                            video: video.clone(),
                            is_active: idx == current_index(),
                        }
                    }
                }
            }
        }
    }
}
