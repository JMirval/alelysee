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
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut id_token = use_context::<Signal<Option<String>>>();
    let lang = crate::use_lang()();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        spawn(async move {
            error.set(None);

            match api::signin(email(), password()).await {
                Ok(token) => {
                    // Store in localStorage
                    let _ = document::eval(&format!(
                        r#"(function(){{
                            try {{ localStorage.setItem("alelysee_id_token", "{}"); }} catch(e) {{}}
                            return "";
                        }})()"#,
                        js_escape(&token)
                    ))
                    .await;

                    // Update context
                    id_token.set(Some(token));

                    // Navigate to /me
                    let _ = document::eval("window.location.assign('/me')").await;
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.signin.title")} }
            p { {crate::t(lang, "auth.signin.body")} }

            form { onsubmit: on_submit,
                if let Some(err) = error() {
                    p { class: "error", {err} }
                }

                div { class: "form-group",
                    label { r#for: "email", {crate::t(lang, "auth.signin.email")} }
                    input {
                        r#type: "email",
                        id: "email",
                        name: "email",
                        required: true,
                        value: "{email}",
                        oninput: move |e| email.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label { r#for: "password", {crate::t(lang, "auth.signin.password")} }
                    input {
                        r#type: "password",
                        id: "password",
                        name: "password",
                        required: true,
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                }

                button { class: "btn primary", r#type: "submit", {crate::t(lang, "auth.signin.submit")} }
            }

            p { class: "hint",
                a { href: "/auth/reset-password", {crate::t(lang, "auth.signin.forgot_password")} }
            }

            p { class: "hint",
                {crate::t(lang, "auth.signin.no_account")}
                " "
                a { href: "/auth/signup", {crate::t(lang, "auth.signin.signup_link")} }
            }

            // OAuth temporarily disabled - uncomment when fixed
            // match cfg() {
            //     None => rsx! { p { {crate::t(lang, "common.loading")} } },
            //     Some(Err(err)) => rsx! { p { class: "error", {err} } },
            //     Some(Ok(cfg)) => { ... }
            // }
        }
    }
}

#[component]
pub fn SignUpForm() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let lang = crate::use_lang()();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        spawn(async move {
            error.set(None);

            // Validate
            if !email().contains('@') {
                error.set(Some(crate::t(lang, "auth.error.invalid_email").to_string()));
                return;
            }

            if password() != confirm_password() {
                error.set(Some(crate::t(lang, "auth.error.passwords_dont_match").to_string()));
                return;
            }

            // Call signup
            match api::signup(email(), password()).await {
                Ok(_) => {
                    success.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.signup.title")} }
            p { {crate::t(lang, "auth.signup.body")} }

            if success() {
                p { class: "success", {crate::t(lang, "auth.signup.success")} }
                p {
                    a { href: "/auth/signin", {crate::t(lang, "auth.signup.signin_link")} }
                }
            } else {
                form { onsubmit: on_submit,
                    if let Some(err) = error() {
                        p { class: "error", {err} }
                    }

                    div { class: "form-group",
                        label { r#for: "email", {crate::t(lang, "auth.signup.email")} }
                        input {
                            r#type: "email",
                            id: "email",
                            name: "email",
                            required: true,
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "password", {crate::t(lang, "auth.signup.password")} }
                        input {
                            r#type: "password",
                            id: "password",
                            name: "password",
                            required: true,
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "confirm_password", {crate::t(lang, "auth.signup.confirm_password")} }
                        input {
                            r#type: "password",
                            id: "confirm_password",
                            name: "confirm_password",
                            required: true,
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }

                    button { class: "btn primary", r#type: "submit", {crate::t(lang, "auth.signup.submit")} }
                }

                p { class: "hint",
                    {crate::t(lang, "auth.signup.already_have_account")}
                    " "
                    a { href: "/auth/signin", {crate::t(lang, "auth.signup.signin_link")} }
                }
            }
        }
    }
}

#[component]
pub fn VerifyEmailPage() -> Element {
    let mut status = use_signal(|| "loading".to_string());
    let mut error_msg = use_signal(|| String::new());
    let lang = crate::use_lang()();

    // Extract token from URL
    use_effect(move || {
        spawn(async move {
            // Get token from query params
            let query = document::eval("window.location.search").await;
            let query = query
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            // Parse token from ?token=xxx
            let token = query
                .strip_prefix("?token=")
                .or_else(|| {
                    query.split('&').find_map(|pair| {
                        pair.strip_prefix("token=")
                    })
                })
                .unwrap_or("");

            if token.is_empty() {
                status.set("error".to_string());
                error_msg.set("No verification token provided".to_string());
                return;
            }

            // Call verify_email
            match api::verify_email(token.to_string()).await {
                Ok(_) => {
                    status.set("success".to_string());
                }
                Err(e) => {
                    status.set("error".to_string());
                    error_msg.set(e.to_string());
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.verify.title")} }

            if status() == "loading" {
                p { {crate::t(lang, "common.loading")} }
            } else if status() == "success" {
                p { class: "success", {crate::t(lang, "auth.verify.success")} }
                p {
                    a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "auth.verify.signin_link")} }
                }
            } else {
                p { class: "error", {crate::t(lang, "auth.verify.error")} }
                if !error_msg().is_empty() {
                    p { class: "hint", {error_msg()} }
                }
                p {
                    a { class: "btn", href: "/auth/signup", {crate::t(lang, "auth.signup.title")} }
                }
            }
        }
    }
}

#[component]
pub fn RequestPasswordResetForm() -> Element {
    let mut email = use_signal(|| String::new());
    let mut submitted = use_signal(|| false);
    let lang = crate::use_lang()();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        spawn(async move {
            // Always succeeds (security: don't reveal if email exists)
            let _ = api::request_password_reset(email()).await;
            submitted.set(true);
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.reset.title")} }
            p { {crate::t(lang, "auth.reset.body")} }

            if submitted() {
                p { class: "success", {crate::t(lang, "auth.reset.success")} }
                p {
                    a { href: "/auth/signin", {crate::t(lang, "auth.reset.back_to_signin")} }
                }
            } else {
                form { onsubmit: on_submit,
                    div { class: "form-group",
                        label { r#for: "email", {crate::t(lang, "auth.reset.email")} }
                        input {
                            r#type: "email",
                            id: "email",
                            name: "email",
                            required: true,
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }

                    button { class: "btn primary", r#type: "submit", {crate::t(lang, "auth.reset.submit")} }
                }

                p { class: "hint",
                    a { href: "/auth/signin", {crate::t(lang, "auth.reset.back_to_signin")} }
                }
            }
        }
    }
}

#[component]
pub fn ResetPasswordConfirmForm() -> Element {
    let mut password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut token = use_signal(|| String::new());
    let lang = crate::use_lang()();

    // Extract token from URL
    use_effect(move || {
        spawn(async move {
            let query = document::eval("window.location.search").await;
            let query = query
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            let tok = query
                .strip_prefix("?token=")
                .or_else(|| {
                    query.split('&').find_map(|pair| {
                        pair.strip_prefix("token=")
                    })
                })
                .unwrap_or("");

            token.set(tok.to_string());
        });
    });

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        spawn(async move {
            error.set(None);

            if password() != confirm_password() {
                error.set(Some(crate::t(lang, "auth.error.passwords_dont_match").to_string()));
                return;
            }

            match api::reset_password(token(), password()).await {
                Ok(_) => {
                    success.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.reset_confirm.title")} }

            if success() {
                p { class: "success", {crate::t(lang, "auth.reset_confirm.success")} }
                p {
                    a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "auth.verify.signin_link")} }
                }
            } else {
                form { onsubmit: on_submit,
                    if let Some(err) = error() {
                        p { class: "error", {err} }
                    }

                    div { class: "form-group",
                        label { r#for: "password", {crate::t(lang, "auth.reset_confirm.password")} }
                        input {
                            r#type: "password",
                            id: "password",
                            name: "password",
                            required: true,
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "confirm_password", {crate::t(lang, "auth.reset_confirm.confirm_password")} }
                        input {
                            r#type: "password",
                            id: "confirm_password",
                            name: "confirm_password",
                            required: true,
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }

                    button { class: "btn primary", r#type: "submit", {crate::t(lang, "auth.reset_confirm.submit")} }
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
    // OAuth implicit flow returns: #id_token=...&access_token=...&...
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
