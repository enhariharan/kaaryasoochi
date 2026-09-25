use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::limits::{FULL_NAME_MAX, PAGE_SIZE_OPTIONS, PASSWORD_MAX, PASSWORD_MIN};
use kaaryasoochi_core::{DateFormat, Language, Layout, TabOrientation, Theme, TimeFormat};

use super::{clean_err, ErrorBanner};
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
        Some(p) if p.contains("%p") => "12-hour",
        Some(_) => "24-hour",
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

#[component]
pub fn Settings() -> Element {
    let s = app_state();
    let error = use_signal(|| None::<String>);
    let cfg = s.settings.read().clone();

    rsx! {
        section { class: "card",
            h1 { {t("settings.title")} }
            ErrorBanner { msg: error }
            div { class: "settings-grid",
                label { {t("settings.theme")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { theme: Theme::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in Theme::ALL { option { key: "{v.as_str()}", value: v.as_str(), selected: *v == cfg.theme, {t(&format!("settings.theme.{}", v.as_str()))} } } } }
                label { {t("settings.language")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { language: Language::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in Language::ALL { option { key: "{v.code()}", value: v.code(), selected: v == cfg.language, "{v.native_name()}" } } } }
                label { {t("settings.tab_orientation")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { tab_orientation: TabOrientation::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in TabOrientation::ALL { option { key: "{v.as_str()}", value: v.as_str(), selected: *v == cfg.tab_orientation, {t(&format!("settings.{}", v.as_str()))} } } } }
                label { {t("settings.job_layout")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { job_layout: Layout::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in Layout::ALL { option { key: "{v.as_str()}", value: v.as_str(), selected: *v == cfg.job_layout, {t(&format!("settings.{}", v.as_str()))} } } } }
                label { {t("settings.date_format")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { date_format: DateFormat::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in DateFormat::ALL { option { key: "{v.as_str()}", value: v.as_str(), selected: *v == cfg.date_format,
                            {date_label(*v)} } } } }
                label { {t("settings.time_format")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { time_format: TimeFormat::parse(&e.value()), ..cfg.clone() }, error) },
                        for v in TimeFormat::ALL { option { key: "{v.as_str()}", value: v.as_str(), selected: *v == cfg.time_format,
                            {time_label(*v)} } } } }
                label { {t("settings.page_size")}
                    select { onchange: { let cfg = cfg.clone(); move |e: FormEvent| save(s.settings, UserSettings { page_size: e.value().parse().unwrap_or(cfg.page_size), ..cfg.clone() }, error) },
                        for v in PAGE_SIZE_OPTIONS { option { key: "{v}", value: "{v}", selected: v == cfg.page_size, "{v}" } } } }
            }
        }
        Profile {}
        ChangePassword {}
        section { class: "card",
            h2 { {t("settings.passkeys")} }
            p { class: "muted", "Coming soon." }
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
                    msg.set(Some(match api::update_full_name(token(), name()).await { Ok(_) => "Saved".into(), Err(e) => clean_err(e) }));
                });
            },
                label { {t("settings.full_name")}
                    input { r#type: "text", maxlength: FULL_NAME_MAX as i64, value: "{name}", oninput: move |e| name.set(e.value()) } }
                if let Some(m) = msg() { p { class: "muted", "{m}" } }
                button { class: "primary", r#type: "submit", {t("common.save")} }
            }
        }
    }
}

#[component]
fn ChangePassword() -> Element {
    let mut s = app_state();
    let (mut current, mut new) = (use_signal(String::new), use_signal(String::new));
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
                label { {t("auth.password")}
                    input { r#type: "password", autocomplete: "current-password", required: true, value: "{current}", oninput: move |e| current.set(e.value()) } }
                label { {t("settings.change_password")}
                    input { r#type: "password", autocomplete: "new-password", required: true, minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64, value: "{new}", oninput: move |e| new.set(e.value()) } }
                ErrorBanner { msg }
                button { class: "primary", r#type: "submit", {t("common.save")} }
            }
        }
    }
}
