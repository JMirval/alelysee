use dioxus::prelude::*;

const AUTH_CSS: Asset = asset!("/assets/styling/auth.css");
const FEED_CSS: Asset = asset!("/assets/styling/feed.css");

/// Provide a best-effort bootstrap that loads a saved id_token (if present)
/// and stores it into the shared `Signal<Option<String>>` context.
///
/// Platforms should provide this context at the app root:
/// `use_context_provider(|| use_signal(|| None::<String>));`
#[component]
pub fn AuthBootstrap() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();

    // Best-effort: try to load from localStorage (web + webviews). If it fails, do nothing.
    // This runs after mount to avoid SSR/hydration mismatches.
    use_effect(move || {
        spawn(async move {
            // Dioxus 0.7: `eval` is available cross-platform (web/desktop/mobile) via WebView JS.
            if let Ok(v) = document::eval(
                r#"(function(){
                    try { return localStorage.getItem("alelysee_id_token") || ""; }
                    catch(e) { return ""; }
                })()"#,
            )
            .await
            {
                if let Some(saved) = v.as_str() {
                    if !saved.trim().is_empty() {
                        id_token.set(Some(saved.to_string()));
                    }
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }
    }
}

#[component]
pub fn AuthGate(children: Element) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let lang = crate::use_lang()();

    if id_token().is_none() {
        return rsx! {
            div { class: "auth_gate",
                h2 { {crate::t(lang, "auth.required")} }
                p { {crate::t(lang, "auth.required.body")} }
                a { class: "btn", href: "/auth/signin", {crate::t(lang, "auth.required.cta")} }
            }
        };
    }

    rsx! {
        {children}

    }
}

#[component]
pub fn SignIn() -> Element {
    // Fetch runtime config from the server so we don't rely on compile-time env vars.
    let cfg = use_resource(|| async move { api::public_config().await });
    let lang = crate::use_lang()();

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.signin.title")} }
            p { {crate::t(lang, "auth.signin.body")} }

            match cfg() {
                None => rsx! {

                    p { {crate::t(lang, "common.loading")} }
                },
                Some(Err(err)) => rsx! {
                    p { class: "error", {format!("{} {err}", crate::t(lang, "auth.config_error_prefix"))} }
                },
                Some(Ok(cfg)) => {
                    let authorize_url = format!(
                        "{}/oauth2/authorize?client_id={}&response_type=token&scope=openid+email+profile&redirect_uri={}",
                        cfg.cognito_domain.trim_end_matches('/'),
                        urlencoding::encode(&cfg.cognito_client_id),
                        urlencoding::encode(&cfg.cognito_redirect_uri),
                    );
                    rsx! {
                        a { class: "btn primary", href: "{authorize_url}", {crate::t(lang, "auth.signin.continue")} }
                        p { class: "hint", {crate::t(lang, "auth.signin.hint")} }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AuthCallback() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();
    let lang = crate::use_lang()();

    // Read location.hash and extract id_token.
    use_effect(move || {
        spawn(async move {
            let hash = document::eval("window.location.hash").await;
            let hash = hash
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            if let Some(token) = extract_id_token_from_hash(&hash) {
                // Persist in localStorage if available.
                let _ = document::eval(&format!(
                    r#"(function(){{
                        try {{ localStorage.setItem("alelysee_id_token", "{}"); }} catch(e) {{}}
                        return "";
                    }})()"#,
                    js_escape(&token)
                ))
                .await;

                id_token.set(Some(token));

                // Navigate to /me
                let _ = document::eval("window.location.assign('/me')").await;
            }
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }
        div { class: "auth_callback",
            h1 { {crate::t(lang, "auth.callback.title")} }
            p {
                {crate::t(lang, "auth.callback.body.prefix")}
                a { href: "/me", "/me" }
                {crate::t(lang, "auth.callback.body.suffix")}
            }
        }
    }
}

#[component]
pub fn SignOutButton() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();
    let lang = crate::use_lang()();
    rsx! {
        button {
            class: "btn",
            onclick: move |_| {
                id_token.set(None);
                spawn(async move {
                    let _ = document::eval(
                            r#"(function(){ try { localStorage.removeItem("alelysee_id_token"); } catch(e) {} return ""; })()"#,
                        )
                        .await;
                });
            },
            {crate::t(lang, "nav.signout")}
        }
    }
}

#[component]
pub fn MePage() -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let lang = crate::use_lang()();

    let me = use_resource(move || {
        let token = token.clone();
        async move {
            if token.trim().is_empty() {
                return Err(ServerFnError::new("Not signed in"));
            }
            api::auth_me(token).await
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }
        document::Link { rel: "stylesheet", href: FEED_CSS }

        div { class: "auth_gate",
            h2 { {crate::t(lang, "me.title")} }
            if id_token().is_none() {
                p { {crate::t(lang, "me.signed_out")} }
                a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "me.signin")} }
            } else {
                SignOutButton {}
                match me() {
                    None => rsx! {
                        p { {crate::t(lang, "common.loading")} }
                    },
                    Some(Err(err)) => rsx! {
                        p { class: "error", {format!("{} {err}", crate::t(lang, "auth.auth_error_prefix"))} }
                    },
                    Some(Ok(me)) => rsx! {
                        p {
                            {crate::t(lang, "me.user_id")}
                            " "
                            code { "{me.user.id}" }
                        }
                        if me.profile_complete {
                            if let Some(p) = me.profile {
                                p { class: "hint", {format!("{} {}", crate::t(lang, "me.signed_in_as"), p.display_name)} }
                            } else {
                                p { class: "hint", {crate::t(lang, "me.profile_complete")} }
                            }
                        } else {
                            p { class: "hint", {crate::t(lang, "me.profile_incomplete")} }
                            a { class: "btn", href: "/me/edit", {crate::t(lang, "me.complete_profile")} }
                        }
                    },
                }
            }
        }

        if id_token().is_some() {
            crate::ActivityFeed {}
        }
    }
}

pub(crate) fn extract_id_token_from_hash(hash: &str) -> Option<String> {
    // Cognito implicit flow returns: #id_token=...&access_token=...&...
    let hash = hash.strip_prefix('#').unwrap_or(hash);
    for pair in hash.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        if k == "id_token" && !v.is_empty() {
            return Some(urlencoding::decode(v).ok()?.into_owned());
        }
    }
    None
}

pub(crate) fn js_escape(s: &str) -> String {
    // Minimal JS string escape for embedding into a double-quoted string.
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_id_token_from_hash() {
        let h = "#id_token=abc123&access_token=zzz&token_type=Bearer";
        assert_eq!(extract_id_token_from_hash(h).as_deref(), Some("abc123"));
    }

    #[test]
    fn extracts_id_token_url_decoded() {
        let h = "#id_token=a%2Bb%3Dc&x=y";
        assert_eq!(extract_id_token_from_hash(h).as_deref(), Some("a+b=c"));
    }

    #[test]
    fn js_escape_quotes_and_backslashes() {
        let s = r#"a"b\c"#;
        assert_eq!(js_escape(s), r#"a\"b\\c"#);
    }
}
