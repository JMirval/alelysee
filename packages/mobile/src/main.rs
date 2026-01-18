use dioxus::prelude::*;
use std::env;

use views::{
    AuthCallback, AuthResetConfirm, AuthResetPassword, AuthSignIn, AuthSignUp, AuthVerify, Blog,
    Home, Me, ProfileEdit, ProgramDetail, ProgramNew, Programs, ProposalDetail, ProposalNew,
    Proposals,
};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MobileNavbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
    #[route("/auth/signin")]
    AuthSignIn {},
    #[route("/auth/signup")]
    AuthSignUp {},
    #[route("/auth/verify?:token")]
    AuthVerify { token: Option<String> },
    #[route("/auth/reset-password")]
    AuthResetPassword {},
    #[route("/auth/reset-password/confirm")]
    AuthResetConfirm {},
    #[route("/auth/callback")]
    AuthCallback {},
    #[route("/me")]
    Me {},
    #[route("/me/edit")]
    ProfileEdit {},
    #[route("/proposals")]
    Proposals {},
    #[route("/proposals/new")]
    ProposalNew {},
    #[route("/proposals/:id")]
    ProposalDetail { id: String },
    #[route("/programs")]
    Programs {},
    #[route("/programs/new")]
    ProgramNew {},
    #[route("/programs/:id")]
    ProgramDetail { id: String },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    install_panic_hook();

    // Initialize tracing for server logs
    #[cfg(feature = "server")]
    init_tracing();

    // Initialize AppState for server
    #[cfg(feature = "server")]
    init_server_state();

    log_runtime_config();
    dioxus::launch(App);
}

#[cfg(feature = "server")]
fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[cfg(feature = "server")]
fn init_server_state() {
    use std::sync::Arc;
    use tokio::runtime::Runtime as TokioRuntime;

    api::config::load_dotenv();

    // Load configuration from environment
    let config = match api::config::AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize AppState
    let state = TokioRuntime::new()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            match api::state::AppState::from_config(config).await {
                Ok(state) => Arc::new(state),
                Err(e) => {
                    eprintln!("Failed to initialize AppState: {}", e);
                    eprintln!("Failed to initialize AppState (debug): {e:?}");
                    std::process::exit(1);
                }
            }
        });

    // Set global state
    api::state::AppState::set_global(state);
    eprintln!("✓ Server initialization complete");
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("panic: {info}");
    }));
}

fn log_runtime_config() {
    let ip = env::var("IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "<missing>".to_string());

    eprintln!("startup: IP={ip} PORT={port}");
    eprintln!("startup: DATABASE_URL={}", redact_db_url(&database_url));

    if database_url.contains("127.0.0.1") || database_url.contains("localhost") {
        eprintln!("startup: WARNING DATABASE_URL points to localhost; this will fail in Railway");
    }

    log_missing_envs(
        "auth",
        &[
            "AUTH_AUTHORIZE_URL",
            "AUTH_CLIENT_ID",
            "AUTH_REDIRECT_URI",
            "AUTH_ISSUER",
            "AUTH_JWKS_URL",
        ],
    );
    log_missing_envs(
        "storage",
        &[
            "STORAGE_BUCKET",
            "STORAGE_ENDPOINT",
            "STORAGE_REGION",
            "STORAGE_ACCESS_KEY",
            "STORAGE_SECRET_KEY",
        ],
    );
}

fn redact_db_url(value: &str) -> String {
    if value == "<missing>" {
        return value.to_string();
    }

    if let Some((prefix, rest)) = value.split_once("://") {
        if let Some((creds, host)) = rest.split_once('@') {
            let user = creds.split(':').next().unwrap_or("user");
            return format!("{prefix}://{user}:***@{host}");
        }
    }

    "<invalid DATABASE_URL>".to_string()
}

fn log_missing_envs(group: &str, keys: &[&str]) {
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|key| env::var(key).ok().is_none())
        .collect();
    if missing.is_empty() {
        return;
    }

    eprintln!(
        "startup: WARNING missing {group} envs: {}",
        missing.join(", ")
    );
}

#[component]
fn App() -> Element {
    // Build cool things ✌️
    let id_token = use_signal(|| None::<String>);
    use_context_provider(|| id_token);
    let auth_ready = use_signal(|| false);
    use_context_provider(|| auth_ready);

    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        ui::CivicTheme {}
        ui::I18nProvider {
            ui::AuthBootstrap {}
            Router::<Route> {}
        }
    }
}

/// A mobile-specific Router around the shared `Navbar` component
/// which allows us to use the mobile-specific `Route` enum.
#[component]
fn MobileNavbar() -> Element {
    // #region agent log
    // H_UI1/H_UI2: verify theme + nav renders on mobile webview
    use_effect(|| {
        spawn(async move {
            let _ = document::eval(r#"
                fetch('http://127.0.0.1:7242/ingest/fe6830f3-3a3a-4330-a18a-e3080ce8e219',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'packages/mobile/src/main.rs:MobileNavbar',message:'mobile_nav_render',data:{platform:'mobile',path:window.location.pathname,width:window.innerWidth},timestamp:Date.now(),sessionId:'debug-session',runId:'ui-theme',hypothesisId:'H_UI1'})}).catch(()=>{});
                return '';
            "#).await;
        });
    });
    // #endregion

    let lang = ui::use_lang()();

    rsx! {
        div { class: "civic_nav",
            div { class: "civic_nav_inner",
                a { class: "brand", href: "/",
                    span { class: "brand_mark" }
                    span { class: "brand_name", {ui::t(lang, "app.name")} }
                }
                div { class: "nav_links",
                    Link { class: "nav_link", to: Route::Proposals {},
                        {ui::t(lang, "nav.proposals")}
                    }
                    Link { class: "nav_link", to: Route::Programs {}, {ui::t(lang, "nav.programs")} }
                    ui::AccountMenu {}
                }
            }
        }
        div { class: "civic_container route_view", Outlet::<Route> {} }
    }
}
