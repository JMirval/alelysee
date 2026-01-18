use dioxus::prelude::*;

const FEED_CSS: Asset = asset!("/assets/styling/feed.css");

#[component]
pub fn ProposalListPage() -> Element {
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();
    let proposals = use_resource(|| async move { api::list_proposals(50).await });
    let mut load_error = use_signal(|| None::<String>);

    use_effect(move || {
        let err = proposals().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts.error(
                    crate::t(lang, "toast.load_proposals_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                h1 { {crate::t(lang, "proposals.title")} }
                a { class: "btn primary", href: "/proposals/new", {crate::t(lang, "proposals.new")} }
            }

            match proposals() {
                None => rsx! {
                    for _ in 0..5 {
                        div { class: "card skeleton",
                            div { class: "card_top",
                                h3 { {crate::t(lang, "common.loading")} }
                                span { class: "score", "…" }
                            }
                            p { class: "summary", "…" }
                        }
                    }
                },
                Some(Err(_)) => rsx! { p { class: "hint", {crate::t(lang, "common.error_try_again")} } },
                Some(Ok(items)) => rsx! {
                    if items.is_empty() {
                        p { class: "hint", {crate::t(lang, "common.no_proposals_yet")} }
                    }
                    for p in items {
                        a { class: "card", href: "/proposals/{p.id}",
                            div { class: "card_top",
                                h3 { "{p.title}" }
                                span { class: "score", "{p.vote_score} votes" }
                            }
                            if !p.summary.trim().is_empty() {
                                p { class: "summary", "{p.summary}" }
                            } else {
                                p { class: "summary", "{truncate(&p.body_markdown, 140)}" }
                            }
                            if !p.tags.is_empty() {
                                div { class: "tags",
                                    for t in p.tags {
                                        span { class: "tag", "{t}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProposalNewPage() -> Element {
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    let mut title = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut tags = use_signal(String::new);
    let mut status = use_signal(String::new);

    let title_ph = crate::t(lang, "proposals.form.title_ph");
    let summary_ph = crate::t(lang, "proposals.form.summary_ph");
    let body_ph = crate::t(lang, "proposals.form.body_ph");
    let tags_ph = crate::t(lang, "proposals.form.tags_ph");

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }

        div { class: "page",
            div { class: "page_header",
                h1 { {crate::t(lang, "proposals.new")} }
                a { class: "btn", href: "/proposals", {crate::t(lang, "common.back")} }
            }

            if id_token().is_none() {
                div { class: "panel",
                    p { {crate::t(lang, "proposals.need_signin_create")} }
                    a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "common.signin")} }
                }
            } else {
                div { class: "panel",
                    label { {crate::t(lang, "proposals.form.title")} }
                    input {
                        value: "{title}",
                        oninput: move |e| title.set(e.value()),
                        placeholder: "{title_ph}",
                    }
                    label { {crate::t(lang, "proposals.form.summary_opt")} }
                    input {
                        value: "{summary}",
                        oninput: move |e| summary.set(e.value()),
                        placeholder: "{summary_ph}",
                    }
                    label { {crate::t(lang, "proposals.form.body")} }
                    textarea {
                        value: "{body}",
                        oninput: move |e| body.set(e.value()),
                        placeholder: "{body_ph}",
                        rows: 10,
                    }
                    label { {crate::t(lang, "proposals.form.tags")} }
                    input {
                        value: "{tags}",
                        oninput: move |e| tags.set(e.value()),
                        placeholder: "{tags_ph}",
                    }
                    button {
                        class: "btn primary",
                        onclick: move |_| {
                            let token = token.clone();
                            let t = title();
                            let s = summary();
                            let b = body();
                            let tg = tags();
                            let lang = lang;
                            let toasts = toasts.clone();
                            spawn(async move {
                                match api::create_proposal(token, t, s, b, tg).await {
                                    Ok(p) => status.set(format!("{} /proposals/{}", crate::t(lang, "proposals.created_open"), p.id)),
                                    Err(e) => toasts.error(
                                        crate::t(lang, "toast.create_proposal_title"),
                                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                                    ),
                                }
                            });
                        },
                        {crate::t(lang, "proposals.form.create")}
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
pub fn ProposalDetailPage(id: String) -> Element {
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();
    let proposal = use_resource(move || {
        let id = id.clone();
        async move { api::get_proposal(id).await }
    });
    let mut load_error = use_signal(|| None::<String>);

    use_effect(move || {
        let err = proposal().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts.error(
                    crate::t(lang, "toast.load_proposal_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                a { class: "btn", href: "/proposals", {crate::t(lang, "common.back")} }
                a { class: "btn", href: "/programs/new", {crate::t(lang, "proposals.bundle_into_program")} }
            }
            match proposal() {
                None => rsx! { p { {crate::t(lang, "common.loading")} } },
                Some(Err(_)) => rsx! { p { class: "hint", {crate::t(lang, "common.error_try_again")} } },
                Some(Ok(p)) => rsx! {
                    div { class: "panel",
                        h1 { "{p.title}" }
                        div { class: "meta",
                            span { class: "score", "{p.vote_score} votes" }
                            span { class: "hint", {format!("{} {}", crate::t(lang, "common.id"), p.id)} }
                        }
                        if !p.summary.trim().is_empty() {
                            p { class: "summary", "{p.summary}" }
                        }
                        pre { class: "body", "{p.body_markdown}" }
                    }
                    div { class: "panel",
                        h2 { {crate::t(lang, "common.vote")} }
                        crate::VoteWidget {
                            target_type: api::types::ContentTargetType::Proposal,
                            target_id: p.id.to_string(),
                            initial_score: p.vote_score,
                        }
                    }
                    crate::CommentThread {
                        target_type: api::types::ContentTargetType::Proposal,
                        target_id: p.id.to_string(),
                    }
                    crate::VideoSection {
                        target_type: api::types::ContentTargetType::Proposal,
                        target_id: p.id.to_string(),
                    }
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}
