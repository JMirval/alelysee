use dioxus::prelude::*;

use api::types::ContentTargetType;

#[component]
pub fn VoteWidget(
    target_type: ContentTargetType,
    target_id: String,
    initial_score: i64,
) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let lang = crate::use_lang()();

    let mut score = use_signal(move || initial_score);
    let mut my_vote = use_signal(|| None::<i16>);
    let mut err = use_signal(String::new);
    let target_id_initial = target_id.clone();
    let mut target_key = use_signal(move || target_id_initial.clone());
    let target_id_value = target_id.clone();
    let target_id_up = target_id.clone();
    let target_id_down = target_id.clone();
    let target_id_clear = target_id;

    if target_key() != target_id_value {
        target_key.set(target_id_value.clone());
        score.set(initial_score);
        my_vote.set(None);
        err.set(String::new());
    }

    use_effect(move || {
        let token = id_token();
        let tid = target_key();
        let initial_score = initial_score;
        spawn(async move {
            if let Some(token) = token {
                match api::get_vote_state(token, target_type, tid).await {
                    Ok(state) => {
                        score.set(state.score);
                        my_vote.set(state.my_vote);
                    }
                    Err(e) => err.set(format!("{e}")),
                }
            } else {
                score.set(initial_score);
                my_vote.set(None);
            }
        });
    });

    rsx! {
        div { class: "vote_widget",
            div { class: "vote_row",
                button {
                    class: "btn",
                    onclick: move |_| {
                        let token = id_token().unwrap_or_default();
                        if token.trim().is_empty() {
                            err.set(crate::t(lang, "common.signin_to_vote"));
                            return;
                        }
                        err.set(String::new());

                        // optimistic toggle
                        let current = my_vote();
                        let desired = if current == Some(1) { 0 } else { 1 };
                        let mut next_score = score();
                        if let Some(c) = current {

                            next_score -= c as i64;
                        }
                        if desired != 0 {
                            next_score += desired as i64;
                        }
                        score.set(next_score);
                        my_vote.set(if desired == 0 { None } else { Some(desired) });
                        let tid = target_id_up.clone();
                        spawn(async move {
                            match api::set_vote(token, target_type, tid, desired).await {
                                Ok(state) => {
                                    score.set(state.score);
                                    my_vote.set(state.my_vote);
                                }
                                Err(e) => err.set(format!("{e}")),
                            }
                        });
                    },
                    "▲"
                }
                div { class: "vote_score", "{score}" }
                button {
                    class: "btn",
                    onclick: move |_| {
                        let token = id_token().unwrap_or_default();
                        if token.trim().is_empty() {
                            err.set(crate::t(lang, "common.signin_to_vote"));
                            return;
                        }
                        err.set(String::new());

                        // optimistic toggle
                        let current = my_vote();
                        let desired = if current == Some(-1) { 0 } else { -1 };
                        let mut next_score = score();
                        if let Some(c) = current {

                            next_score -= c as i64;
                        }
                        if desired != 0 {
                            next_score += desired as i64;
                        }
                        score.set(next_score);
                        my_vote.set(if desired == 0 { None } else { Some(desired) });
                        let tid = target_id_down.clone();
                        spawn(async move {
                            match api::set_vote(token, target_type, tid, desired).await {
                                Ok(state) => {
                                    score.set(state.score);
                                    my_vote.set(state.my_vote);
                                }
                                Err(e) => err.set(format!("{e}")),
                            }
                        });
                    },
                    "▼"
                }
                button {
                    class: "btn",
                    onclick: move |_| {
                        let token = id_token().unwrap_or_default();
                        if token.trim().is_empty() {
                            err.set(crate::t(lang, "common.signin_to_vote"));
                            return;
                        }
                        err.set(String::new());
                        // optimistic clear
                        let current = my_vote();
                        let mut next_score = score();
                        if let Some(c) = current {
                            next_score -= c as i64;
                        }
                        score.set(next_score);
                        my_vote.set(None);

                        let tid = target_id_clear.clone();
                        spawn(async move {
                            match api::set_vote(token, target_type, tid, 0).await {
                                Ok(state) => {
                                    score.set(state.score);
                                    my_vote.set(state.my_vote);
                                }
                                Err(e) => err.set(format!("{e}")),
                            }
                        });
                    },
                    {crate::t(lang, "vote.clear")}
                }
            }
            if let Some(v) = my_vote() {
                p { class: "hint", {format!("{} {v}", crate::t(lang, "vote.your_vote"))} }
            }
            if !err().is_empty() {
                p { class: "error", "{err}" }
            }
        }
    }
}
