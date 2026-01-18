use dioxus::prelude::*;

const FEED_CSS: Asset = asset!("/assets/styling/feed.css");

#[component]
pub fn ProfileEditPage() -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    let mut display_name = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut avatar_url = use_signal(String::new);
    let mut location = use_signal(String::new);
    let mut status = use_signal(String::new);

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                h1 { "Edit profile" }
                a { class: "btn", href: "/me", "Back" }
            }

            if id_token().is_none() {
                div { class: "panel",
                    p { "You need to sign in to edit your profile." }
                    a { class: "btn primary", href: "/auth/signin", "Sign in" }
                }
            } else {
                div { class: "panel",
                    label { "Display name" }
                    input { value: "{display_name}", oninput: move |e| display_name.set(e.value()) }
                    label { "Bio" }
                    textarea { value: "{bio}", oninput: move |e| bio.set(e.value()), rows: 6 }
                    label { "Avatar URL (optional)" }
                    input { value: "{avatar_url}", oninput: move |e| avatar_url.set(e.value()) }
                    label { "Location (optional)" }
                    input { value: "{location}", oninput: move |e| location.set(e.value()) }

                    button {
                        class: "btn primary",
                        onclick: move |_| {
                            let token = token.clone();
                            let dn = display_name();
                            let b = bio();
                            let av = avatar_url();
                            let loc = location();
                            let lang = lang;
                            let toasts = toasts.clone();
                            spawn(async move {
                                match api::upsert_profile(
                                    token,
                                    dn,
                                    b,
                                    if av.trim().is_empty() { None } else { Some(av) },
                                    if loc.trim().is_empty() { None } else { Some(loc) },
                                )
                                .await {
                                    Ok(_) => status.set("Saved.".to_string()),
                                    Err(e) => toasts.error(
                                        crate::t(lang, "toast.profile_save_title"),
                                        Some(format!(
                                            "{} {e}",
                                            crate::t(lang, "toast.details")
                                        )),
                                    ),
                                }
                            });
                        },
                        "Save"
                    }

                    if !status().is_empty() {
                        p { class: "hint", "{status}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ActivityFeed() -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    let feed = use_resource(move || {
        let token = token.clone();
        async move {
            if token.trim().is_empty() {
                return Ok(vec![]);
            }
            api::list_my_activity(token, 50).await
        }
    });
    let mut load_error = use_signal(|| None::<String>);

    use_effect(move || {
        let err = feed().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts.error(
                    crate::t(lang, "toast.load_activity_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "panel",
            h2 { "Your activity" }
            match feed() {
                None => rsx! { p { "Loading…" } },
                Some(Err(_)) => rsx! { p { class: "hint", {crate::t(lang, "common.error_try_again")} } },
                Some(Ok(items)) => rsx! {
                    if items.is_empty() {
                        p { class: "hint", "No activity yet." }
                    }
                    for a in items {
                        div { class: "activity",
                            span { class: "hint", "{a.created_at}" }
                            span { " " }
                            span { class: "hint", "{a.action:?}" }
                            span { " " }
                            span { class: "hint", "{a.target_type:?}" }
                            if let Some(title) = a.title {
                                span { " — " }
                                span { "{title}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
