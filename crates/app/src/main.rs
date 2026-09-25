// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

mod api;
#[cfg(feature = "server")]
mod backend;
#[cfg(feature = "server")]
mod fsbrowse;
mod pages;
mod state;

use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::Theme;
use pages::*;
use state::{app_state, AppState};

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/jobs/new")]
    JobNew {},
    #[route("/jobs/:id")]
    JobEdit { id: i32 },
    #[route("/settings")]
    Settings {},
}

fn main() {
    // Desktop/mobile clients have no same-origin server, so point them at one.
    // Read at build time: `KAARYASOOCHI_SERVER_URL=https://host:port dx build ...`.
    #[cfg(not(any(feature = "web", feature = "server")))]
    dioxus::fullstack::set_server_url(
        option_env!("KAARYASOOCHI_SERVER_URL").unwrap_or("http://127.0.0.1:8080"),
    );
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let state = AppState {
        token: use_signal(|| None),
        settings: use_signal(UserSettings::default),
        system_tz: use_signal(|| "UTC".to_string()),
        system_date: use_signal(Default::default),
        system_time: use_signal(Default::default),
        ready: use_signal(|| false),
    };
    use_context_provider(|| state);

    // Restore the session (if any), device locale and the user's settings once at startup.
    use_future(move || async move {
        let mut s = state;
        let loc = state::detect_locale().await;
        s.system_tz.set(loc.timezone);
        s.system_date.set(loc.date);
        s.system_time.set(loc.time);
        if let Some(tok) = state::stored_token().await {
            match api::get_settings(tok.clone()).await {
                Ok(cfg) => {
                    s.settings.set(cfg);
                    s.token.set(Some(tok));
                }
                Err(_) => state::persist_token(None), // expired or revoked
            }
        }
        s.ready.set(true);
    });

    let theme = match state.settings.read().theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
        Theme::System => "system",
    };
    // Text direction and language come from the user's language (its locale file declares `@dir`).
    // <html> gets them too so scrollbars, native pickers and page margins mirror as well.
    let lang = state.settings.read().language;
    use_effect(move || {
        let l = state.settings.read().language;
        let _ = document::eval(&format!(
            "document.documentElement.setAttribute('dir','{}');document.documentElement.setAttribute('lang','{}')",
            l.dir(),
            l.code()
        ));
    });

    rsx! {
        document::Title { "Kaaryasoochi" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "app", "data-theme": theme, dir: lang.dir(), lang: lang.code(),
            if *state.ready.read() { Router::<Route> {} } else { p { class: "muted center", "…" } }
        }
    }
}

/// Authenticated layout: redirects to login without a session, otherwise renders nav + page.
#[component]
fn Shell() -> Element {
    let mut s = app_state();
    let nav = use_navigator();
    use_effect(move || {
        if s.token.read().is_none() {
            nav.replace(Route::Login {});
        }
    });
    // Let the server know the device's zone; it is used while the time-zone setting is "system".
    use_effect(move || {
        if let Some(tok) = s.token.read().clone() {
            let tz = s.system_tz.read().clone();
            spawn(async move {
                let _ = api::report_system_timezone(tok, tz).await;
            });
        }
    });
    if s.token.read().is_none() {
        return rsx! {};
    }

    rsx! {
        header { class: "topbar",
            Link { to: Route::Home {}, class: "brand", {state::t("app.name")} }
            nav {
                Link { to: Route::Home {}, {state::t("home.title")} }
                Link { to: Route::Settings {}, {state::t("settings.title")} }
                button { class: "link", onclick: move |_| {
                    let tok = state::token();
                    spawn(async move { let _ = api::logout(tok).await; });
                    state::persist_token(None);
                    s.token.set(None);
                    s.settings.set(UserSettings::default());
                }, {state::t("auth.logout")} }
            }
        }
        main { Outlet::<Route> {} }
    }
}
