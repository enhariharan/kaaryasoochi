// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::limits::{
    FULL_NAME_MAX, PAGE_SIZE_OPTIONS, PASSWORD_MAX, PASSWORD_MIN, SECRET_ANSWER_MAX,
    SECRET_QUESTION_MAX, SECRET_QUESTION_MIN,
};
use kaaryasoochi_core::timezone;
use kaaryasoochi_core::{ButtonDisplay, DateFormat, Language, Layout, TabOrientation, Theme, TimeFormat};

use super::{clean_err, ErrorBanner, IBtn, PasswordField, Pick};
use crate::api;
use crate::icons::*;
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
    // The account's secret question, needed by both the change-password and secret sections. A
    // just-saved question overrides the loaded one so the page updates without a reload.
    let me = use_resource(move || async move { api::me(token()).await });
    let mut saved_question = use_signal(|| None::<String>);
    // Arrived from the "Set it now" banner: bring the secret-question section into view and focus
    // it. The section renders after the account has loaded, so the script waits for it (up to 4 s).
    use_effect(move || {
        if *s.focus_secret.read() {
            let mut flag = s.focus_secret;
            let _ = document::eval(
                "(async () => { for (let i = 0; i < 40; i++) { const el = document.getElementById('secret-section'); \
                 if (el) { el.scrollIntoView({behavior: 'smooth', block: 'start'}); \
                 el.querySelector('input')?.focus({preventScroll: true}); return; } \
                 await new Promise(r => setTimeout(r, 100)); } })()",
            );
            flag.set(false);
        }
    });
    let secret = saved_question().or_else(|| {
        me.read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|u| u.secret_question.clone())
    });

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
                label { {t("settings.button_display")}
                    Pick { id: "set-btndisplay", value: cfg.button_display.as_str().to_string(),
                        options: opts(ButtonDisplay::ALL, |v| v.as_str().into(), |v| t(&format!("settings.button_display.{}", v.as_str())).into()),
                        onchange: { let cfg = cfg.clone(); move |v: String| save(s.settings, UserSettings { button_display: ButtonDisplay::parse(&v), ..cfg.clone() }, error) } } }
            }
        }
        Profile {}
        match secret {
            None => rsx! { section { class: "card", p { class: "muted", "…" } } },
            Some(question) => rsx! {
                ChangePassword { question: question.clone() }
                SecretQuestion { question, on_saved: move |q: String| {
                    saved_question.set(Some(q));
                    let mut missing = s.secret_missing;
                    missing.set(false);
                } }
            },
        }
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
                IBtn { icon: icon_check(), label: t("common.save"), r#type: "submit", class: "primary" }
            }
        }
    }
}

/// Changing the password needs the current password, the answer to the secret question and the
/// new password twice. The server enforces all of it; the form just collects it.
#[component]
fn ChangePassword(question: String) -> Element {
    let mut s = app_state();
    let (current, answer) = (use_signal(String::new), use_signal(String::new));
    let (new, confirm) = (use_signal(String::new), use_signal(String::new));
    let mut msg = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    if question.is_empty() {
        // No secret yet: nothing to verify against, so point at the section below.
        return rsx! {
            section { class: "card",
                h2 { {t("settings.change_password")} }
                p { class: "muted", {t("settings.secret.not_set")} }
            }
        };
    }
    rsx! {
        section { class: "card",
            h2 { {t("settings.change_password")} }
            form { onsubmit: move |e| {
                e.prevent_default();
                if new() != confirm() {
                    msg.set(Some(t("auth.password_mismatch").to_string()));
                    return;
                }
                busy.set(true);
                msg.set(None);
                spawn(async move {
                    match api::change_password(token(), current(), answer(), new(), confirm()).await {
                        // The server revokes all sessions on a password change, so sign out locally too.
                        Ok(()) => { crate::state::persist_token(None); s.token.set(None); }
                        Err(e) => msg.set(Some(clean_err(e))),
                    }
                    busy.set(false);
                });
            },
                PasswordField { label: "auth.password", value: current, autocomplete: "current-password" }
                div { class: "question",
                    small { class: "muted", {t("settings.secret.your_question")} }
                    p { dir: "auto", "{question}" }
                }
                PasswordField { label: "auth.secret_answer", value: answer, autocomplete: "off", maxlength: SECRET_ANSWER_MAX as i64 }
                PasswordField { label: "auth.new_password", value: new, autocomplete: "new-password",
                    minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64 }
                PasswordField { label: "auth.confirm_password", value: confirm, autocomplete: "new-password",
                    minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64 }
                ErrorBanner { msg }
                IBtn { icon: icon_check(), label: t("common.save"), r#type: "submit", class: "primary", disabled: busy() }
            }
        }
    }
}

/// Change the secret question and answer. Needs the current password and, when a secret already
/// exists, its current answer.
#[component]
fn SecretQuestion(question: String, on_saved: EventHandler<String>) -> Element {
    let (current_password, current_answer) = (use_signal(String::new), use_signal(String::new));
    let mut new_question = use_signal(String::new);
    let new_answer = use_signal(String::new);
    let mut msg = use_signal(|| None::<String>);
    let mut done = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let has_secret = !question.is_empty();
    rsx! {
        section { class: "card", id: "secret-section",
            h2 { {t("settings.secret.title")} }
            if has_secret {
                div { class: "question",
                    small { class: "muted", {t("settings.secret.your_question")} }
                    p { dir: "auto", "{question}" }
                }
            } else {
                p { class: "muted", {t("settings.secret.not_set")} }
            }
            form { onsubmit: move |e| {
                e.prevent_default();
                busy.set(true);
                msg.set(None);
                done.set(false);
                spawn(async move {
                    match api::set_secret_question(token(), current_password(), current_answer(), new_question(), new_answer()).await {
                        Ok(q) => {
                            let (mut cp, mut ca, mut na) = (current_password, current_answer, new_answer);
                            cp.set(String::new());
                            ca.set(String::new());
                            na.set(String::new());
                            new_question.set(String::new());
                            done.set(true);
                            on_saved.call(q);
                        }
                        Err(e) => msg.set(Some(clean_err(e))),
                    }
                    busy.set(false);
                });
            },
                PasswordField { label: "auth.password", value: current_password, autocomplete: "current-password" }
                if has_secret {
                    PasswordField { label: "settings.secret.current_answer", value: current_answer, autocomplete: "off", maxlength: SECRET_ANSWER_MAX as i64 }
                }
                label { {t("settings.secret.new_question")}
                    input { r#type: "text", dir: "auto", required: true, minlength: SECRET_QUESTION_MIN as i64, maxlength: SECRET_QUESTION_MAX as i64,
                        value: "{new_question}", oninput: move |e| new_question.set(e.value()) }
                    small { class: "muted", {t("auth.secret_hint")} } }
                PasswordField { label: "settings.secret.new_answer", value: new_answer, autocomplete: "off", maxlength: SECRET_ANSWER_MAX as i64 }
                ErrorBanner { msg }
                if done() { p { class: "muted", role: "status", {t("common.saved")} } }
                IBtn { icon: icon_check(), label: t("common.save"), r#type: "submit", class: "primary", disabled: busy() }
            }
        }
    }
}
