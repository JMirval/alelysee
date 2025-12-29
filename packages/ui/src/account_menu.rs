use dioxus::prelude::*;

const THEME_CSS: Asset = asset!("/assets/styling/theme.css");
const JS_SIGN_OUT_CLEAR: &str =
    r#"(function(){ try { localStorage.removeItem("heliastes_id_token"); } catch(e) {} return ""; })()"#;

/// Account control in the top-right of the app nav:
/// - logged out: "Sign in / Sign up" button
/// - logged in: avatar button → dropdown (Profile, Edit profile, Sign out)
#[component]
pub fn AccountMenu() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    let lang_sig = crate::use_lang();
    let lang = lang_sig();

    let mut open = use_signal(|| false);

    let me = use_resource(move || {
        let token = token.clone();
        async move {
            if token.trim().is_empty() {
                return Ok(None);
            }
            api::auth_me(token).await.map(Some)
        }
    });

    let on_sign_out = move |_| {
        id_token.set(None);
        open.set(false);
        spawn(async move {
            let _ = document::eval(JS_SIGN_OUT_CLEAR).await;
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: THEME_CSS }

        if id_token().is_none() {
            a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "nav.signin")} }
        } else {
            div { class: "account_menu",
                button {
                    class: "avatar_btn",
                    onclick: move |_| {
                        let next = !open();
                        open.set(next);
                    },
                    match me() {
                        None => rsx! { span { class: "avatar_fallback", "?" } },
                        Some(Ok(None)) => rsx! { span { class: "avatar_fallback", "?" } },
                        Some(Ok(Some(me))) => {
                            let avatar_url = me.profile.as_ref().and_then(|p| p.avatar_url.clone());
                            let initials = me.profile.as_ref()
                                .map(|p| initials(&p.display_name))
                                .unwrap_or_else(|| "U".to_string());
                            rsx! {
                                if let Some(url) = avatar_url {
                                    img { class: "avatar_img", src: "{url}", alt: "Avatar" }
                                } else {
                                    span { class: "avatar_fallback", "{initials}" }
                                }
                            }
                        }
                        Some(Err(_)) => rsx! { span { class: "avatar_fallback", "!" } },
                    }
                }

                if open() {
                    div { class: "dropdown",
                        a { class: "dropdown_item", href: "/me", onclick: move |_| open.set(false), {crate::t(lang, "nav.profile")} }
                        a { class: "dropdown_item", href: "/me/edit", onclick: move |_| open.set(false), {crate::t(lang, "nav.edit_profile")} }
                        div { class: "dropdown_item",
                            span { class: "hint", {crate::t(lang, "lang.label")} }
                            div { style: "margin-left:auto; display:flex; gap:6px;",
                                button { class: "btn", onclick: move |_| crate::set_lang(crate::Lang::Fr), "FR" }
                                button { class: "btn", onclick: move |_| crate::set_lang(crate::Lang::En), "EN" }
                            }
                        }
                        button { class: "dropdown_item danger", onclick: on_sign_out, {crate::t(lang, "nav.signout")} }
                    }
                }
            }
        }
    }
}

fn initials(name: &str) -> String {
    let mut parts = name.split_whitespace().filter(|p| !p.is_empty());
    let a = parts.next().and_then(|s| s.chars().next());
    let b = parts.next().and_then(|s| s.chars().next());
    match (a, b) {
        (Some(a), Some(b)) => format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_uppercase()),
        (Some(a), None) => format!("{}", a.to_ascii_uppercase()),
        _ => "U".to_string(),
    }
}


