use dioxus::prelude::*;

const AUTH_CSS: Asset = asset!("/assets/styling/auth.css");
const FEED_CSS: Asset = asset!("/assets/styling/feed.css");
const BOOKMARKS_CSS: Asset = asset!("/assets/styling/bookmarks.css");

/// Provide a best-effort bootstrap that loads a saved id_token (if present)
/// and stores it into the shared `Signal<Option<String>>` context.
///
/// Platforms should provide this context at the app root:
/// `use_context_provider(|| use_signal(|| None::<String>));`
#[component]
pub fn AuthBootstrap() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();
    let mut auth_ready = use_context::<Signal<bool>>();

    // Best-effort: try to load from localStorage (web + webviews). If it fails, do nothing.
    // This runs after mount to avoid SSR/hydration mismatches.
    use_effect(move || {
        spawn(async move {
            if let Some(saved) = read_id_token_from_storage() {
                id_token.set(Some(saved));
                auth_ready.set(true);
                return;
            }

            #[cfg(not(target_arch = "wasm32"))]
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
            auth_ready.set(true);
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
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut id_token = use_context::<Signal<Option<String>>>();
    let mut show_resend = use_signal(|| false);
    let mut resend_pending = use_signal(|| false);
    let navigator = use_navigator();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();
    let toasts_submit = toasts.clone();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        show_resend.set(false);
        let navigator = navigator;
        let toasts = toasts_submit.clone();
        spawn(async move {
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

                    // Navigate to /me without full reload so in-memory auth stays intact.
                    navigator.push("/me");
                }
                Err(e) => {
                    let message = e.to_string();
                    if message.to_lowercase().contains("verify your email") {
                        show_resend.set(true);
                    }
                    toasts.error(
                        crate::t(lang, "toast.signin_failed_title"),
                        Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                    );
                }
            }
        });
    };

    let toasts_resend = toasts.clone();
    let on_resend = move |_| {
        if resend_pending() {
            return;
        }
        let toasts = toasts_resend.clone();
        resend_pending.set(true);
        let lang = lang;
        let email = email();
        spawn(async move {
            match api::resend_verification_email(email).await {
                Ok(()) => {
                    toasts.success(
                        crate::t(lang, "auth.resend.title"),
                        Some(crate::t(lang, "auth.resend.body")),
                    );
                }
                Err(e) => {
                    toasts.error(
                        crate::t(lang, "auth.resend.failed_title"),
                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                    );
                }
            }
            resend_pending.set(false);
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }

        div { class: "auth_signin",
            h1 { {crate::t(lang, "auth.signin.title")} }
            p { {crate::t(lang, "auth.signin.body")} }

            form { onsubmit: on_submit,
                div { class: "form-group",
                    label { r#for: "email", {crate::t(lang, "auth.signin.email")} }
                    input {
                        r#type: "email",
                        id: "email",
                        name: "email",
                        required: true,
                        value: "{email}",
                        oninput: move |e| {
                            show_resend.set(false);
                            email.set(e.value());
                        },
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
                        oninput: move |e| {
                            show_resend.set(false);
                            password.set(e.value());
                        },
                    }
                }

                button { class: "btn primary", r#type: "submit",
                    {crate::t(lang, "auth.signin.submit")}
                }
            }

            p { class: "hint",
                a { href: "/auth/reset-password", {crate::t(lang, "auth.signin.forgot_password")} }
            }

            if show_resend() {
                div { class: "hint",
                    p { {crate::t(lang, "auth.resend.prompt")} }
                    button {
                        class: "btn",
                        r#type: "button",
                        disabled: resend_pending(),
                        onclick: on_resend,
                        if resend_pending() {
                            {crate::t(lang, "auth.resend.sending")}
                        } else {
                            {crate::t(lang, "auth.resend.cta")}
                        }
                    }
                }
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
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut success = use_signal(|| false);
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        let toasts = toasts.clone();
        spawn(async move {
            // Validate
            if !email().contains('@') {
                toasts.error(
                    crate::t(lang, "toast.signup_failed_title"),
                    Some(crate::t(lang, "auth.error.invalid_email")),
                );
                return;
            }

            if password() != confirm_password() {
                toasts.error(
                    crate::t(lang, "toast.signup_failed_title"),
                    Some(crate::t(lang, "auth.error.passwords_dont_match")),
                );
                return;
            }

            // Call signup
            match api::signup(email(), password()).await {
                Ok(_) => {
                    success.set(true);
                }
                Err(e) => {
                    toasts.error(
                        crate::t(lang, "toast.signup_failed_title"),
                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                    );
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
                        label { r#for: "confirm_password",
                            {crate::t(lang, "auth.signup.confirm_password")}
                        }
                        input {
                            r#type: "password",
                            id: "confirm_password",
                            name: "confirm_password",
                            required: true,
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }

                    button { class: "btn primary", r#type: "submit",
                        {crate::t(lang, "auth.signup.submit")}
                    }
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
pub fn VerifyEmailPage(token: Option<String>) -> Element {
    let mut status = use_signal(|| "loading".to_string());
    let mut error_msg = use_signal(String::new);
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();
    let token = token.unwrap_or_default();

    use_effect(move || {
        let token = token.clone();
        let toasts = toasts.clone();
        spawn(async move {
            if token.is_empty() {
                status.set("error".to_string());
                error_msg.set("No verification token provided".to_string());
                toasts.error(
                    crate::t(lang, "toast.verify_failed_title"),
                    Some(crate::t(lang, "toast.try_again")),
                );
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
                    toasts.error(
                        crate::t(lang, "toast.verify_failed_title"),
                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                    );
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
                    a { class: "btn primary", href: "/auth/signin",
                        {crate::t(lang, "auth.verify.signin_link")}
                    }
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
    let mut email = use_signal(String::new);
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

                    button { class: "btn primary", r#type: "submit",
                        {crate::t(lang, "auth.reset.submit")}
                    }
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
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut success = use_signal(|| false);
    let mut token = use_signal(String::new);
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

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
                    query
                        .split('&')
                        .find_map(|pair| pair.strip_prefix("token="))
                })
                .unwrap_or("");

            token.set(tok.to_string());
        });
    });

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        let toasts = toasts.clone();
        spawn(async move {
            if password() != confirm_password() {
                toasts.error(
                    crate::t(lang, "toast.reset_failed_title"),
                    Some(crate::t(lang, "auth.error.passwords_dont_match")),
                );
                return;
            }

            match api::reset_password(token(), password()).await {
                Ok(_) => {
                    success.set(true);
                }
                Err(e) => {
                    toasts.error(
                        crate::t(lang, "toast.reset_failed_title"),
                        Some(format!("{} {e}", crate::t(lang, "toast.details"))),
                    );
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
                    a { class: "btn primary", href: "/auth/signin",
                        {crate::t(lang, "auth.verify.signin_link")}
                    }
                }
            } else {
                form { onsubmit: on_submit,
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
                        label { r#for: "confirm_password",
                            {crate::t(lang, "auth.reset_confirm.confirm_password")}
                        }
                        input {
                            r#type: "password",
                            id: "confirm_password",
                            name: "confirm_password",
                            required: true,
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                    }

                    button { class: "btn primary", r#type: "submit",
                        {crate::t(lang, "auth.reset_confirm.submit")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn AuthCallback() -> Element {
    let mut id_token = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let lang = crate::use_lang()();

    // Read location.hash and extract id_token.
    use_effect(move || {
        let navigator = navigator;
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

                // Navigate to /me without full reload so in-memory auth stays intact.
                navigator.push("/me");
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
    let mut id_token = use_context::<Signal<Option<String>>>();
    let auth_ready = try_use_context::<Signal<bool>>();
    let lang = crate::use_lang()();
    let toasts = crate::use_toasts();

    use_effect(move || {
        if id_token().is_some() {
            return;
        }
        spawn(async move {
            if let Some(saved) = read_id_token_from_storage() {
                id_token.set(Some(saved));
                return;
            }

            #[cfg(not(target_arch = "wasm32"))]
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

    let me = use_resource(move || {
        let token = id_token().unwrap_or_default();
        async move {
            if token.trim().is_empty() {
                return Err(ServerFnError::new("Not signed in"));
            }
            api::auth_me(token).await
        }
    });
    let mut load_error = use_signal(|| None::<String>);

    use_effect(move || {
        let err = me().and_then(|res| res.err()).map(|e| e.to_string());
        if err.as_ref() != load_error().as_ref() {
            if let Some(message) = &err {
                toasts.error(
                    crate::t(lang, "toast.me_load_title"),
                    Some(format!("{} {message}", crate::t(lang, "toast.details"))),
                );
            }
            load_error.set(err);
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: AUTH_CSS }
        document::Link { rel: "stylesheet", href: FEED_CSS }
        document::Link { rel: "stylesheet", href: BOOKMARKS_CSS }

        div { class: "auth_gate",
            h2 { {crate::t(lang, "me.title")} }
            if auth_ready.as_ref().is_some_and(|ready| !ready()) {
                p { {crate::t(lang, "common.loading")} }
            } else if id_token().is_none() {
                p { {crate::t(lang, "me.signed_out")} }
                a { class: "btn primary", href: "/auth/signin", {crate::t(lang, "me.signin")} }
            } else {
                SignOutButton {}
                match me() {
                    None => rsx! {
                        p { {crate::t(lang, "common.loading")} }
                    },
                    Some(Err(_)) => rsx! {
                        p { class: "hint", {crate::t(lang, "common.error_try_again")} }
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
            ProfileTabs {}
        }
    }
}

#[component]
fn ProfileTabs() -> Element {
    let mut active_tab = use_signal(|| "activity");

    rsx! {
        div { class: "profile-tabs",
            button {
                class: if active_tab() == "activity" { "tab active" } else { "tab" },
                onclick: move |_| active_tab.set("activity"),
                "Activity"
            }
            button {
                class: if active_tab() == "bookmarks" { "tab active" } else { "tab" },
                onclick: move |_| active_tab.set("bookmarks"),
                "Bookmarks"
            }
        }

        match active_tab() {
            "activity" => rsx! {
                crate::ActivityFeed {}
            },
            "bookmarks" => rsx! {
                BookmarksSection {}
            },
            _ => rsx! {}
        }
    }
}

#[component]
fn BookmarksSection() -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();

    let mut bookmarks = use_signal(Vec::<api::types::Video>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);
    let offset = use_signal(|| 0i64);

    // Load bookmarks
    use_effect(move || {
        let token = token.clone();
        spawn(async move {
            loading.set(true);
            match api::list_bookmarked_videos(token, 20, offset()).await {
                Ok(vids) => {
                    bookmarks.set(vids);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "bookmarks-section",
            h2 { "Bookmarked Videos ({bookmarks().len()})" }

            if loading() {
                p { "Loading bookmarks..." }
            } else if let Some(err) = error_msg() {
                p { class: "error", "Error: {err}" }
            } else if bookmarks().is_empty() {
                div { class: "empty-state",
                    p { "You haven't bookmarked any videos yet" }
                    p { class: "hint", "Discover videos to save your favorites" }
                    a { href: "/videos", class: "btn primary", "Explore Videos" }
                }
            } else {
                div { class: "bookmarks-grid",
                    for video in bookmarks() {
                        BookmarkCard {
                            key: "{video.id}",
                            video: video,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BookmarkCard(video: api::types::Video) -> Element {
    let id_token = use_context::<Signal<Option<String>>>();
    let token = id_token().unwrap_or_default();
    let cfg = use_resource(|| async move { api::public_config().await });
    let mut show_remove = use_signal(|| false);

    let on_remove = move |_| {
        let token = token.clone();
        let video_id = video.id.to_string();
        spawn(async move {
            let _ = api::bookmark_video(token, video_id).await;
            // TODO: Refresh bookmarks list
        });
    };

    rsx! {
        div {
            class: "bookmark-card",
            onmouseenter: move |_| show_remove.set(true),
            onmouseleave: move |_| show_remove.set(false),

            a { href: "/videos/{video.id}",
                match cfg() {
                    None => rsx! { div { class: "video-thumbnail", "Loading..." } },
                    Some(Err(_)) => rsx! { div { class: "video-thumbnail", "Error" } },
                    Some(Ok(cfg)) => {
                        let src = cfg.media_base_url.as_ref().map(|base| {
                            format!("{}/{}", base.trim_end_matches('/'), video.storage_key)
                        });

                        rsx! {
                            if let Some(src) = src {
                                video {
                                    class: "video-thumbnail",
                                    src: "{src}",
                                    preload: "metadata",
                                }
                            } else {
                                div { class: "video-thumbnail placeholder",
                                    "▶️"
                                }
                            }
                        }
                    }
                }

                div { class: "video-info",
                    div { class: "video-score", "{video.vote_score} votes" }
                    if let Some(duration) = video.duration_seconds {
                        div { class: "video-duration", "{duration}s" }
                    }
                }
            }

            if show_remove() {
                button {
                    class: "remove-btn",
                    onclick: on_remove,
                    "Remove"
                }
            }
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

#[cfg(target_arch = "wasm32")]
fn read_id_token_from_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let token = storage.get_item("alelysee_id_token").ok()??;
    if token.trim().is_empty() {
        None
    } else {
        Some(token)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_id_token_from_storage() -> Option<String> {
    None
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
