// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::limits::{FULL_NAME_MAX, PAGE_SIZE_OPTIONS, PASSWORD_MAX, PASSWORD_MIN};
use kaaryasoochi_core::timezone;
use kaaryasoochi_core::{DateFormat, Language, Layout, TabOrientation, Theme, TimeFormat};

use super::{clean_err, ErrorBanner, PasswordField, Pick};
use crate::api;
use crate::state::{app_state, t, token};

fn date_label(v: DateFormat) -> String {
    match v.pattern() {
        Some(p) => p
            .replace("%Y", "YYYY")
            .replace("%m", "MM")
            .replace("%d", "DD")
            .replace("%b", "Mon"),
        None => t("settings.system_default").to_string(),
    }
}

fn time_label(v: TimeFormat) -> &'static str {
    match v.pattern() {
        Some(p) if p.contains("%p") => t("settings.time_12h"),
        Some(_) => t("settings.time_24h"),
        None => t("settings.system_default"),
    }
}

/// Persist a settings change immediately and mirror it into app state.
fn save(mut settings: Signal<UserSettings>, next: UserSettings, mut error: Signal<Option<String>>) {
    let prev = settings();
    settings.set(next.clone()); // optimistic so theme/language switch instantly
    spawn(async move {
        match api::update_settings(token(), next).await {
            Ok(saved) => {
                settings.set(saved);
                error.set(None);
            }
            Err(e) => {
                settings.set(prev);
                error.set(Some(clean_err(e)));
            }
        }
    });
}

fn opts<T: Copy>(
    all: &[T],
    value: impl Fn(T) -> String,
    label: impl Fn(T) -> String,
) -> Vec<(String, String)> {
    all.iter().map(|v| (value(*v), label(*v))).collect()
}

#[component]
pub fn Settings() -> Element {
    let s = app_state();
    let error = use_signal(|| None::<String>);
    let cfg = s.settings.read().clone();

    let mut timezones = vec![(
        timezone::SYSTEM.to_string(),
        format!("{} ({})", t("settings.system_default"), s.system_tz.read()),
    )];
    timezones.extend(timezone::all_names().map(|n| (n.to_string(), n.to_string())));
    let sizes: Vec<(String, String)> = PAGE_SIZE_OPTIONS
        .iter()
        .map(|v| (v.to_string(), v.to_string()))
        .collect();

    rsx! {
        section { class: "card",
            h1 { {t("settings.title")} }
            ErrorBanner { msg: error }
            div { class: "settings-grid",
                label { {t("settings.theme")}
                    Pick { id: "set-theme", value: cfg.theme.as_str().to_string(),
                        options: opts(Theme::ALL, |v| v.as_str().into(), |v| t(&format!("settings.theme.{}", v.as_str())).into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { theme: Theme::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.language")}
                    Pick { id: "set-language", value: cfg.language.code().to_string(),
                        options: opts(&Language::ALL, |v| v.code().into(), |v| v.native_name().into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { language: Language::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.timezone")}
                    Pick { id: "set-timezone", value: cfg.timezone.clone(), options: timezones,
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { timezone: v, ..cfg.clone() }, error) } } }
                label { {t("settings.tab_orientation")}
                    Pick { id: "set-tabs", value: cfg.tab_orientation.as_str().to_string(),
                        options: opts(TabOrientation::ALL, |v| v.as_str().into(), |v| t(&format!("settings.{}", v.as_str())).into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { tab_orientation: TabOrientation::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.job_layout")}
                    Pick { id: "set-layout", value: cfg.job_layout.as_str().to_string(),
                        options: opts(Layout::ALL, |v| v.as_str().into(), |v| t(&format!("settings.{}", v.as_str())).into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { job_layout: Layout::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.date_format")}
                    Pick { id: "set-date", value: cfg.date_format.as_str().to_string(),
                        options: opts(DateFormat::ALL, |v| v.as_str().into(), date_label),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { date_format: DateFormat::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.time_format")}
                    Pick { id: "set-time", value: cfg.time_format.as_str().to_string(),
                        options: opts(TimeFormat::ALL, |v| v.as_str().into(), |v| time_label(v).into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { time_format: TimeFormat::parse(&v), ..cfg.clone() }, error) } } }
                label { {t("settings.page_size")}
                    Pick { id: "set-pagesize", value: cfg.page_size.to_string(), options: sizes,
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { page_size: v.parse().unwrap_or(cfg.page_size), ..cfg.clone() }, error) } } }
            }
        }
        Profile {}
        ChangePassword {}
        section { class: "card",
            h2 { {t("settings.passkeys")} }
            p { class: "muted", {t("common.coming_soon")} }
        }
    }
}

#[component]
fn Profile() -> Element {
    let mut name = use_signal(String::new);
    let mut msg = use_signal(|| None::<String>);
    let me = use_resource(move || async move {
        let u = api::me(token()).await?;
        name.set(u.full_name.clone());
        Ok::<_, ServerFnError>(u)
    });
    rsx! {
        section { class: "card",
            if let Some(Ok(u)) = &*me.read() { p { class: "muted", "@{u.username}" } }
            form { onsubmit: move |e| {
                e.prevent_default();
                spawn(async move {
                    msg.set(Some(match api::update_full_name(token(), name()).await { Ok(_) => t("common.saved").to_string(), Err(e) => clean_err(e) }));
                });
            },
                label { {t("settings.full_name")}
                    input { r#type: "text", dir: "auto", maxlength: FULL_NAME_MAX as i64, value: "{name}", oninput: move |e| name.set(e.value()) } }
                if let Some(m) = msg() { p { class: "muted", "{m}" } }
                button { class: "primary", r#type: "submit", {t("common.save")} }
            }
        }
    }
}

#[component]
fn ChangePassword() -> Element {
    let mut s = app_state();
    let (current, new) = (use_signal(String::new), use_signal(String::new));
    let mut msg = use_signal(|| None::<String>);
    rsx! {
        section { class: "card",
            h2 { {t("settings.change_password")} }
            form { onsubmit: move |e| {
                e.prevent_default();
                spawn(async move {
                    match api::change_password(token(), current(), new()).await {
                        // The server revokes all sessions on a password change, so sign out locally too.
                        Ok(()) => { crate::state::persist_token(None); s.token.set(None); }
                        Err(e) => msg.set(Some(clean_err(e))),
                    }
                });
            },
                PasswordField { label: "auth.password", value: current, autocomplete: "current-password" }
                PasswordField { label: "settings.change_password", value: new,
                    autocomplete: "new-password", minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64 }
                ErrorBanner { msg }
                button { class: "primary", r#type: "submit", {t("common.save")} }
            }
        }
    }
}
