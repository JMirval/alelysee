use dioxus::prelude::*;

const FEED_CSS: Asset = asset!("/assets/styling/feed.css");

#[component]
pub fn ProgramListPage() -> Element {
    let lang = crate::use_lang()();
    let programs = use_resource(|| async move { api::list_programs(50).await });

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                h1 { {crate::t(lang, "programs.title")} }
                a { class: "btn primary", href: "/programs/new", {crate::t(lang, "programs.new")} }
            }

            match programs() {
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
                Some(Err(err)) => rsx! { p { class: "error", {format!("{} {err}", crate::t(lang, "common.error_prefix"))} } },
                Some(Ok(items)) => rsx! {
                    if items.is_empty() {
                        p { class: "hint", {crate::t(lang, "common.no_programs_yet")} }
                    }
                    for p in items {
                        a { class: "card", href: "/programs/{p.id}",
                            div { class: "card_top",
                                h3 { "{p.title}" }
                                span { class: "score", "{p.vote_score} votes" }
                            }
                            p { class: "summary", "{truncate(&p.summary, 160)}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProgramNewPage() -> Element {
    let lang = crate::use_lang()();
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    let mut title = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut proposal_ids = use_signal(String::new);
    let mut status = use_signal(String::new);

    let title_ph = crate::t(lang, "programs.form.title_ph");
    let summary_ph = crate::t(lang, "programs.form.summary_ph");
    let body_ph = crate::t(lang, "programs.form.body_ph");
    let proposal_ids_ph = crate::t(lang, "programs.form.proposal_ids_ph");

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                h1 { {crate::t(lang, "programs.new")} }
                a { class: "btn", href: "/programs", {crate::t(lang, "common.back")} }
            }

            if id_token().is_none() {
                div { class: "panel",
                    p { {crate::t(lang, "programs.need_signin_create")} }
                    a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "common.signin")} }
                }
            } else {
                div { class: "panel",
                    label { {crate::t(lang, "programs.form.title")} }
                    input {
                        value: "{title}",
                        oninput: move |e| title.set(e.value()),
                        placeholder: "{title_ph}",
                    }
                    label { {crate::t(lang, "programs.form.summary")} }
                    input {
                        value: "{summary}",
                        oninput: move |e| summary.set(e.value()),
                        placeholder: "{summary_ph}",
                    }
                    label { {crate::t(lang, "programs.form.body")} }
                    textarea {
                        value: "{body}",
                        oninput: move |e| body.set(e.value()),
                        placeholder: "{body_ph}",
                        rows: 8,
                    }
                    label { {crate::t(lang, "programs.form.proposal_ids")} }
                    input {
                        value: "{proposal_ids}",
                        oninput: move |e| proposal_ids.set(e.value()),
                        placeholder: "{proposal_ids_ph}",
                    }
                    button {
                        class: "btn primary",
                        onclick: move |_| {
                            let token = token.clone();
                            let t = title();
                            let s = summary();
                            let b = body();
                            let ids = proposal_ids();
                            let lang = lang;
                            spawn(async move {
                                match api::create_program(token.clone(), t, s, b).await {
                                    Ok(program) => {
                                        // best-effort: add items in order
                                        for (pos, id) in ids.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).enumerate() {
                                            let _ = api::add_program_item(token.clone(), program.id.to_string(), id.to_string(), pos as i32).await;
                                        }
                                        status.set(format!("{} /programs/{}", crate::t(lang, "programs.created_open"), program.id));
                                    }
                                    Err(e) => status.set(format!("{} {e}", crate::t(lang, "common.error_prefix"))),
                                }
                            });
                        },
                        {crate::t(lang, "programs.form.create")}
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
pub fn ProgramDetailPage(id: String) -> Element {
    let lang = crate::use_lang()();
    let detail = use_resource(move || {
        let id = id.clone();
        async move { api::get_program(id).await }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: FEED_CSS }
        div { class: "page",
            div { class: "page_header",
                a { class: "btn", href: "/programs", {crate::t(lang, "common.back")} }
                a { class: "btn", href: "/proposals", {crate::t(lang, "programs.browse_proposals")} }
            }
            match detail() {
                None => rsx! { p { {crate::t(lang, "common.loading")} } },
                Some(Err(err)) => rsx! { p { class: "error", {format!("{} {err}", crate::t(lang, "common.error_prefix"))} } },
                Some(Ok(d)) => rsx! {
                    div { class: "panel",
                        h1 { "{d.program.title}" }
                        div { class: "meta",
                            span { class: "score", "{d.program.vote_score} votes" }
                            span { class: "hint", {format!("{} {}", crate::t(lang, "common.id"), d.program.id)} }
                        }
                        if !d.program.summary.trim().is_empty() {
                            p { class: "summary", "{d.program.summary}" }
                        }
                        pre { class: "body", "{d.program.body_markdown}" }
                    }
                    div { class: "panel",
                        h2 { {crate::t(lang, "common.vote")} }
                        crate::VoteWidget {
                            target_type: api::types::ContentTargetType::Program,
                            target_id: d.program.id.to_string(),
                            initial_score: d.program.vote_score,
                        }
                    }
                    crate::CommentThread {
                        target_type: api::types::ContentTargetType::Program,
                        target_id: d.program.id.to_string(),
                    }
                    crate::VideoSection {
                        target_type: api::types::ContentTargetType::Program,
                        target_id: d.program.id.to_string(),
                    }
                    div { class: "panel",
                        h2 { {crate::t(lang, "programs.bundled_proposals")} }
                        if d.proposals.is_empty() {
                            p { class: "hint", {crate::t(lang, "programs.none_bundled")} }
                        }
                        for p in d.proposals {
                            a { class: "card", href: "/proposals/{p.id}",
                                div { class: "card_top",
                                    h3 { "{p.title}" }
                                    span { class: "score", "{p.vote_score} votes" }
                                }
                                p { class: "summary", "{truncate(&p.summary, 160)}" }
                            }
                        }
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
