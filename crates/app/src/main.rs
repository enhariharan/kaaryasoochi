// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

mod api;
#[cfg(feature = "server")]
mod backend;
#[cfg(feature = "server")]
mod fsbrowse;
mod icons;
mod pages;
mod state;

use dioxus::prelude::*;
use icons::*;
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
    #[route("/forgot")]
    Forgot {},
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
    // The server must start scheduling at boot, not when the first client happens to call it:
    // connect, migrate and start the scheduler (which also catches up on runs missed while the
    // machine was off) before serving. A failure here exits, so a supervisor such as systemd
    // restarts the process.
    #[cfg(feature = "server")]
    dioxus::serve(|| async {
        backend::db().await.map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(dioxus::server::router(App))
    });
    #[cfg(not(feature = "server"))]
    {
        // Set program name and GDK class to "kaaryasoochi" so that the Wayland compositor receives
        // that string as the xdg_toplevel app_id, matching kaaryasoochi.desktop.
        // These calls must happen before GTK initialises (which Dioxus does on first event-loop
        // creation), so calling them here — before LaunchBuilder — is correct.
        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::raw::c_char;
            extern "C" {
                fn g_set_prgname(prgname: *const c_char);
                fn gdk_set_program_class(program_class: *const c_char);
            }
            let name = b"kaaryasoochi\0";
            g_set_prgname(name.as_ptr() as *const c_char);
            gdk_set_program_class(name.as_ptr() as *const c_char);
        }
        dioxus::launch(App);
    }
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
        secret_missing: use_signal(|| false),
        focus_secret: use_signal(|| false),
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
        document::Link { rel: "icon", r#type: "image/svg+xml", href: asset!("/assets/icon.svg") }
        document::Link { rel: "icon", r#type: "image/png", sizes: "32x32", href: asset!("/assets/icons/32.png") }
        div { class: "app", "data-theme": theme, "data-display": state.settings.read().button_display.as_str(), dir: lang.dir(), lang: lang.code(),
            if *state.ready.read() { Router::<Route> {} } else { p { class: "muted center", "…" } }
        }
    }
}

/// Authenticated layout: redirects to login without a session, otherwise renders nav + page.
#[component]
fn Shell() -> Element {
    let mut s = app_state();
    let nav = use_navigator();
    let mut help_open = use_signal(|| false);
    use_effect(move || {
        if s.token.read().is_none() {
            nav.replace(Route::Login {});
        }
    });

    // Global keyboard shortcuts. Registered once per mount; a flag prevents double-binding on
    // re-mount. Keys are ignored while any input, textarea or select has focus.
    // Sequences: `g h` → Home, `g s` → Settings.  Singles: `n` → new job, `?` → help.
    use_future(move || async move {
        let mut e = document::eval(r#"
            (function() {
                if (window._ksRegistered) return;
                window._ksRegistered = true;
                var seq = null, timer = null;
                document.addEventListener('keydown', function(ev) {
                    var tag = (document.activeElement || {}).tagName || '';
                    tag = tag.toLowerCase();
                    var editable = !!(document.activeElement && document.activeElement.isContentEditable);
                    if (ev.altKey || ev.ctrlKey || ev.metaKey) return;
                    if (tag === 'input' || tag === 'textarea' || tag === 'select' || editable) return;
                    if (seq === 'g') {
                        clearTimeout(timer); seq = null;
                        dioxus.send('g:' + ev.key);
                        ev.preventDefault();
                        return;
                    }
                    if (ev.key === 'g') {
                        seq = 'g';
                        clearTimeout(timer);
                        timer = setTimeout(function() { seq = null; }, 1500);
                        ev.preventDefault();
                        return;
                    }
                    dioxus.send(':' + ev.key);
                });
            })();
        "#);
        while let Ok(msg) = e.recv::<String>().await {
            match msg.as_str() {
                "g:h" => { let _ = nav.push(Route::Home {}); }
                "g:s" => { let _ = nav.push(Route::Settings {}); }
                ":n" => { let _ = nav.push(Route::JobNew {}); }
                ":?" => { help_open.set(!help_open()); }
                _ => {}
            }
        }
    });
    // Long-touch on any .ibtn shows the full label+hotkey tooltip for 500ms.
    use_effect(move || {
        let _ = document::eval(
            r#"(function(){
                if(window._ltRegistered)return;
                window._ltRegistered=true;
                var t=null,el=null;
                document.addEventListener('touchstart',function(e){
                    var b=e.target.closest('.ibtn');
                    if(!b)return;
                    el=b;
                    t=setTimeout(function(){b.classList.add('touch-expanded');},500);
                },{passive:true});
                function clear(){clearTimeout(t);if(el){el.classList.remove('touch-expanded');el=null;}}
                document.addEventListener('touchend',clear,{passive:true});
                document.addEventListener('touchmove',clear,{passive:true});
            })();"#,
        );
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
    // Older accounts have no secret question; remind them until they set one.
    use_effect(move || {
        if let Some(tok) = s.token.read().clone() {
            let mut missing = s.secret_missing;
            spawn(async move {
                if let Ok(u) = api::me(tok).await {
                    missing.set(u.secret_question.is_empty());
                }
            });
        }
    });
    if s.token.read().is_none() {
        return rsx! {};
    }

    rsx! {
        header { class: "topbar",
            Link { to: Route::Home {}, class: "brand",
                img { src: asset!("/assets/icon.svg"), width: "28", height: "28", alt: "", "aria-hidden": "true" }
                {state::t("app.name")}
            }
            nav {
                ILink { to: Route::Home {}, icon: icon_home(), label: state::t("home.title"), hotkey: "g h" }
                ILink { to: Route::Settings {}, icon: icon_settings(), label: state::t("settings.title"), hotkey: "g s" }
                IBtn { icon: icon_logout(), label: state::t("auth.logout"), hotkey: "Alt+Q",
                    class: "link", accesskey: "q", aria_keyshortcuts: "Alt+q",
                    onclick: move |_| {
                        let tok = state::token();
                        spawn(async move { let _ = api::logout(tok).await; });
                        state::persist_token(None);
                        s.token.set(None);
                        s.settings.set(UserSettings::default());
                    } }
            }
        }
        if *s.secret_missing.read() {
            div { class: "banner", role: "status",
                {state::t("banner.set_secret")} " "
                Link { to: Route::Settings {}, onclick: move |_| s.focus_secret.set(true), {state::t("banner.set_secret_link")} }
            }
        }
        main { Outlet::<Route> {} }
        if help_open() { HotkeyHelp { open: help_open } }
    }
}

#[component]
fn HotkeyHelp(open: Signal<bool>) -> Element {
    let mut open = open;
    // Auto-focus the close button so Escape/Enter can dismiss the overlay without further Tab.
    use_effect(move || {
        let _ = document::eval(
            "requestAnimationFrame(function(){document.querySelector('.hotkey-help .close-btn')?.focus();})",
        );
    });
    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| open.set(false),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key().to_string() == "Escape" { open.set(false); }
            },
            div { class: "modal hotkey-help", role: "dialog", "aria-modal": true,
                "aria-label": state::t("hotkeys.title"),
                onclick: move |e| e.stop_propagation(),
                h2 { {state::t("hotkeys.title")} }
                table { "aria-label": state::t("hotkeys.title"),
                    tbody {
                        tr { class: "section-head", td { colspan: 2, {state::t("home.title")} } }
                        tr { td { kbd { "g" } " " kbd { "h" } } td { {state::t("home.title")} } }
                        tr { td { kbd { "n" } } td { {state::t("job.add")} } }
                        tr { td { kbd { "Alt" } " " kbd { "N" } } td { {state::t("job.add")} } }

                        tr { class: "section-head", td { colspan: 2, {state::t("settings.title")} } }
                        tr { td { kbd { "g" } " " kbd { "s" } } td { {state::t("settings.title")} } }
                        tr { td { kbd { "Alt" } " " kbd { "Q" } } td { {state::t("auth.logout")} } }

                        tr { class: "section-head", td { colspan: 2, {state::t("job.add")} } }
                        tr { td { kbd { "Ctrl" } " " kbd { "↵" } } td { {state::t("common.save")} } }
                        tr { td { kbd { "Alt" } " " kbd { "R" } } td { {state::t("job.run_now")} } }

                        tr { class: "section-head", td { colspan: 2, {state::t("picker.title")} } }
                        tr { td { kbd { "Esc" } } td { {state::t("common.cancel")} } }
                        tr { td { kbd { "⌫" } } td { {state::t("picker.up")} } }

                        tr { class: "section-head", td { colspan: 2, "" } }
                        tr { td { kbd { "?" } } td { {state::t("hotkeys.title")} } }
                    }
                }
                button { class: "primary close-btn", r#type: "button", onclick: move |_| open.set(false),
                    {state::t("common.cancel")} }
            }
        }
    }
}
