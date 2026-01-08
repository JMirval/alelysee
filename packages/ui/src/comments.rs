use dioxus::prelude::*;

use api::types::ContentTargetType;

#[component]
pub fn CommentThread(target_type: ContentTargetType, target_id: String) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();

    let mut draft = use_signal(String::new);
    let mut err = use_signal(String::new);

    let target_id_for_list = target_id.clone();
    let mut comments = use_resource(move || {
        let target_id = target_id_for_list.clone();
        async move { api::list_comments(target_type, target_id, 200).await }
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
                        err.set(String::new());
                        let token = token.clone();
                        let body = draft();
                        let tid = target_id.clone();
                        let lang = lang;
                        spawn(async move {
                            if body.trim().is_empty() {
                                err.set(crate::t(lang, "comments.empty_error"));
                                return;
                            }
                            match api::create_comment(token, target_type, tid, None, body).await {
                                Ok(_) => {
                                    draft.set(String::new());
                                    comments.restart();
                                }
                                Err(e) => err.set(format!("{e}")),
                            }
                        });
                    },
                    {crate::t(lang, "comments.post")}
                }
                if !err().is_empty() {
                    p { class: "error", "{err}" }
                }
            }

            match comments() {
                None => rsx! {
                    p { {crate::t(lang, "common.loading")} }
                },
                Some(Err(e)) => rsx! {
                    p { class: "error", {format!("{} {e}", crate::t(lang, "common.error_prefix"))} }
                },
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
