use dioxus::prelude::*;

use api::types::ContentTargetType;

#[component]
pub fn CommentThread(target_type: ContentTargetType, target_id: String) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    let mut draft = use_signal(String::new);

    let target_id_for_list = target_id.clone();
    let mut comments = use_resource(move || {
        let target_id = target_id_for_list.clone();
        async move { api::list_comments(target_type, target_id, 200).await }
    });
    let mut load_error = use_signal(|| None::<String>);

    let toasts_for_load = toasts.clone();
    use_effect(move || {
        let err = comments().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts_for_load.error(
                    crate::t(lang, "toast.load_comments_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        div { class: "panel",
            h2 { {crate::t(lang, "comments.title")} }

            if id_token().is_none() {
                p { class: "hint", {crate::t(lang, "common.signin_to_comment")} }
                a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "common.signin")} }
            } else {
                textarea {
                    value: "{draft}",
                    oninput: move |e| draft.set(e.value()),
                    placeholder: crate::t(lang, "comments.placeholder"),
                    rows: 4,
                }
                button {
                    class: "btn primary",
                    onclick: move |_| {
                        let token = token.clone();
                        let body = draft();
                        let tid = target_id.clone();
                        let lang = lang;
                        let toasts = toasts.clone();
                        spawn(async move {
                            if body.trim().is_empty() {
                                toasts.error(
                                    crate::t(lang, "toast.create_comment_title"),
                                    Some(crate::t(lang, "comments.empty_error")),
                                );
                                return;
                            }
                            match api::create_comment(token, target_type, tid, None, body).await {
                                Ok(_) => {
                                    draft.set(String::new());
                                    comments.restart();
                                }
                                Err(e) => toasts.error(
                                    crate::t(lang, "toast.create_comment_title"),
                                    Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                                ),
                            }
                        });
                    },
                    {crate::t(lang, "comments.post")}
                }
            }

            match comments() {
                None => rsx! {
                    p { {crate::t(lang, "common.loading")} }
                },
                Some(Err(_)) => rsx! { p { class: "hint", {crate::t(lang, "common.error_try_again")} } },
                Some(Ok(items)) => rsx! {
                    if items.is_empty() {
                        p { class: "hint", {crate::t(lang, "common.no_comments_yet")} }
                    }
                    for c in items {
                        div { class: "comment",
                            div { class: "comment_meta",
                                span { class: "hint", {format!("{} {}", crate::t(lang, "comments.by"), c.author_user_id)} }
                                span { class: "score", "{c.vote_score} votes" }
                            }
                            pre { class: "body", "{c.body_markdown}" }
                        }
                    }
                },
            }
        }
    }
}
