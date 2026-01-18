use dioxus::prelude::*;

use api::types::ContentTargetType;

#[component]
pub fn VideoSection(target_type: ContentTargetType, target_id: String) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    let cfg = use_resource(|| async move { api::public_config().await });
    let target_id_for_list = target_id.clone();
    let mut videos = use_resource(move || {
        let target_id = target_id_for_list.clone();
        async move { api::list_videos(target_type, target_id, 20).await }
    });
    let mut load_error = use_signal(|| None::<String>);

    let mut status = use_signal(String::new);

    let toasts_for_load = toasts.clone();
    use_effect(move || {
        let err = videos().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts_for_load.error(
                    crate::t(lang, "toast.load_videos_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        div { class: "panel",
            h2 { "Videos" }

            match videos() {
                None => rsx! { p { "Loading…" } },
                Some(Err(_)) => rsx! { p { class: "hint", {crate::t(lang, "common.error_try_again")} } },
                Some(Ok(items)) => rsx! {
                    if items.is_empty() {
                        p { class: "hint", "No videos yet." }
                    }
                    for v in items {
                        div { class: "panel",
                            p { class: "hint", "Video id: {v.id}" }
                            div { class: "meta",
                                span { class: "score", "{v.vote_score} votes" }
                                span { class: "hint", "{v.content_type}" }
                            }
                            match cfg() {
                                None => rsx! { p { class: "hint", "Loading player…" } },
                                Some(Err(_)) => rsx! { p { class: "hint", "Player not configured." } },
                                Some(Ok(cfg)) => {
                                    let src = cfg.media_base_url.as_ref().map(|base| {
                                        format!("{}/{}", base.trim_end_matches('/'), v.storage_key)
                                    });
                                    rsx! {
                                        if let Some(src) = src {
                                            video {
                                                class: "video_player",
                                                controls: true,
                                                src: "{src}",
                                            }
                                        } else {
                                            p { class: "hint", "Set MEDIA_BASE_URL to enable playback." }
                                        }
                                    }
                                }
                            }

                            crate::VoteWidget {
                                target_type: ContentTargetType::Video,
                                target_id: v.id.to_string(),
                                initial_score: v.vote_score,
                            }
                            crate::CommentThread {
                                target_type: ContentTargetType::Video,
                                target_id: v.id.to_string(),
                            }
                        }
                    }
                }
            }

            if id_token().is_none() {
                p { class: "hint", "Sign in to upload a video." }
            } else {
                div { class: "panel",
                    label { "Upload a video" }
                    input { id: "alelysee_video_file", r#type: "file", accept: "video/*" }
                    button {
                        class: "btn primary",
                        onclick: move |_| {
                            status.set(String::new());

                            let token = token.clone();
                            let tid = target_id.clone();
                            let toasts = toasts.clone();
                            spawn(async move {
                                // Read file metadata from JS
                                let meta = document::eval(
                                    r#"(function(){
                                        const el = document.getElementById("alelysee_video_file");
                                        if(!el || !el.files || !el.files[0]) return "";
                                        const f = el.files[0];
                                        return String(f.size) + "|" + (f.type || "application/octet-stream");
                                    })()"#,
                                )
                                .await
                                .ok()
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                .unwrap_or_default();

                                if meta.trim().is_empty() {
                                    toasts.error(
                                        crate::t(lang, "toast.video_missing_file_title"),
                                        Some(crate::t(lang, "toast.try_again")),
                                    );
                                    return;
                                }

                                let mut it = meta.splitn(2, '|');
                                let size: i64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                                let ctype = it.next().unwrap_or("application/octet-stream").to_string();

                                let intent = match api::create_video_upload_intent(
                                    token.clone(),
                                    target_type,
                                    tid.clone(),
                                    ctype.clone(),
                                    size,
                                )
                                .await
                                {
                                    Ok(i) => i,
                                    Err(e) => {
                                        toasts.error(
                                            crate::t(lang, "toast.upload_video_title"),
                                            Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                                        );
                                        return;
                                    }
                                };

                                status.set("Uploading to storage…".to_string());

                                // Upload file using fetch(PUT presigned_url, body=file)
                                let js = format!(
                                    r#"(async function(){{
                                        const el = document.getElementById("alelysee_video_file");
                                        if(!el || !el.files || !el.files[0]) return "no_file";
                                        const f = el.files[0];
                                        const resp = await fetch("{}", {{
                                            method: "PUT",
                                            headers: {{ "Content-Type": "{}" }},
                                            body: f
                                        }});
                                        if(!resp.ok) return "upload_failed:" + resp.status;
                                        return "ok";
                                    }})()"#,
                                    js_escape(&intent.presigned_put_url),
                                    js_escape(&ctype),
                                );

                                let upload_res = document::eval(&js)
                                    .await
                                    .ok()
                                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                                    .unwrap_or_else(|| "upload_eval_failed".to_string());

                                if upload_res != "ok" {
                                    toasts.error(
                                        crate::t(lang, "toast.upload_video_title"),
                                        Some(format!("{} {upload_res}", crate::t(lang, "toast.details"))),
                                    );
                                    return;
                                }

                                status.set("Finalizing…".to_string());

                                match api::finalize_video_upload(
                                    token,
                                    target_type,
                                    tid,
                                    intent.storage_key,
                                    ctype,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        status.set("Uploaded.".to_string());
                                        videos.restart();
                                    }
                                    Err(e) => toasts.error(
                                        crate::t(lang, "toast.upload_video_title"),
                                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                                    ),
                                }
                            });
                        },
                        "Upload"
                    }
                    if !status().is_empty() {
                        p { class: "hint", "{status}" }
                    }
                }
            }
        }
    }
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
