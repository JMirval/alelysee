use dioxus::prelude::*;

use views::{
    AuthCallback, AuthSignIn, Blog, Home, Me, ProfileEdit, ProgramDetail, ProgramNew, Programs,
    ProposalDetail, ProposalNew, Proposals,
};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(DesktopNavbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
    #[route("/auth/signin")]
    AuthSignIn {},
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
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Build cool things ✌️
    let id_token = use_signal(|| None::<String>);
    use_context_provider(|| id_token);

    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        ui::CivicTheme {}
        ui::I18nProvider {}
        ui::AuthBootstrap {}

        Router::<Route> {}
    }
}

/// A desktop-specific Router around the shared `Navbar` component
/// which allows us to use the desktop-specific `Route` enum.
#[component]
fn DesktopNavbar() -> Element {
    // #region agent log
    // H_UI1/H_UI2: verify theme + nav renders on desktop webview
    use_effect(|| {
        spawn(async move {
            let _ = document::eval(r#"
                fetch('http://127.0.0.1:7242/ingest/fe6830f3-3a3a-4330-a18a-e3080ce8e219',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({location:'packages/desktop/src/main.rs:DesktopNavbar',message:'desktop_nav_render',data:{platform:'desktop',path:window.location.pathname,width:window.innerWidth},timestamp:Date.now(),sessionId:'debug-session',runId:'ui-theme',hypothesisId:'H_UI1'})}).catch(()=>{});
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
