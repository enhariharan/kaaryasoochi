// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! App-wide client state, provided as context by `App` and read by every page.

use chrono::{DateTime, Duration, Utc};
use dioxus::document::eval;
use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::{tr, DateFormat, TimeFormat};

const TOKEN_KEY: &str = "kaaryasoochi.token";

#[derive(Clone, Copy)]
pub struct AppState {
    pub token: Signal<Option<String>>,
    pub settings: Signal<UserSettings>,
    /// Minutes to add to UTC to get local time.
    pub utc_offset_min: Signal<i32>,
    /// What `System` resolves to on this device.
    pub system_date: Signal<DateFormat>,
    pub system_time: Signal<TimeFormat>,
    /// False until the initial session/settings load has finished.
    pub ready: Signal<bool>,
}

pub fn app_state() -> AppState {
    consume_context::<AppState>()
}

/// Translate a static-text key into the user's language.
pub fn t(key: &str) -> &'static str {
    tr(app_state().settings.read().language, key)
}

/// Token for API calls; empty string simply fails auth server-side.
pub fn token() -> String {
    app_state().token.read().clone().unwrap_or_default()
}

/// Date and time in the user's chosen (or the device's) formats, shifted to local time.
pub fn fmt_datetime(dt: DateTime<Utc>) -> String {
    let s = app_state();
    let local = dt.naive_utc() + Duration::minutes(*s.utc_offset_min.read() as i64);
    let cfg = s.settings.read();
    let d = cfg
        .date_format
        .pattern()
        .or(s.system_date.read().pattern())
        .unwrap_or("%Y-%m-%d");
    let t = cfg
        .time_format
        .pattern()
        .or(s.system_time.read().pattern())
        .unwrap_or("%H:%M");
    local.format(&format!("{d} {t}")).to_string()
}

// ---- browser/webview helpers (all targets render in a webview or a browser) ----

pub async fn stored_token() -> Option<String> {
    let mut e = eval(&format!("dioxus.send(localStorage.getItem('{TOKEN_KEY}'))"));
    e.recv::<Option<String>>().await.ok().flatten()
}

pub fn persist_token(token: Option<&str>) {
    let js = match token {
        Some(t) => format!("localStorage.setItem('{TOKEN_KEY}', {})", serde_json_str(t)),
        None => format!("localStorage.removeItem('{TOKEN_KEY}')"),
    };
    let _ = eval(&js);
}

/// JSON-escape a string for embedding in JS (tokens are hex, but stay safe).
fn serde_json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub struct Locale {
    pub utc_offset_min: i32,
    pub date: DateFormat,
    pub time: TimeFormat,
}

pub async fn detect_locale() -> Locale {
    let mut e = eval(
        r#"
        const parts = new Intl.DateTimeFormat(undefined).formatToParts(new Date(2000, 10, 22));
        const order = parts.filter(p => ['day','month','year'].includes(p.type)).map(p => p.type[0]).join('');
        const h12 = new Intl.DateTimeFormat(undefined, {hour: 'numeric'}).resolvedOptions().hour12 === true;
        dioxus.send([-new Date().getTimezoneOffset(), order, h12]);
    "#,
    );
    match e.recv::<(i32, String, bool)>().await {
        Ok((off, order, h12)) => Locale {
            utc_offset_min: off,
            date: match order.as_str() {
                "dmy" => DateFormat::Dmy,
                "mdy" => DateFormat::Mdy,
                _ => DateFormat::Iso,
            },
            time: if h12 {
                TimeFormat::H12
            } else {
                TimeFormat::H24
            },
        },
        Err(_) => Locale {
            utc_offset_min: 0,
            date: DateFormat::Iso,
            time: TimeFormat::H24,
        },
    }
}

/// `datetime-local` value (`YYYY-MM-DDTHH:MM`, browser-local) -> UTC. DST-correct via JS.
pub async fn local_input_to_utc(v: &str) -> Option<DateTime<Utc>> {
    let mut e = eval(&format!(
        "const d = new Date({}); dioxus.send(isNaN(d) ? null : d.toISOString())",
        serde_json_str(v)
    ));
    let iso = e.recv::<Option<String>>().await.ok().flatten()?;
    DateTime::parse_from_rfc3339(&iso)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// UTC -> `datetime-local` value in browser-local time.
pub async fn utc_to_local_input(dt: DateTime<Utc>) -> String {
    let mut e = eval(&format!(
        "const d = new Date({ms}); const p = n => String(n).padStart(2,'0'); \
         dioxus.send(`${{d.getFullYear()}}-${{p(d.getMonth()+1)}}-${{p(d.getDate())}}T${{p(d.getHours())}}:${{p(d.getMinutes())}}`)",
        ms = dt.timestamp_millis()));
    e.recv::<String>().await.unwrap_or_default()
}

/// `datetime-local` value for "now + 1 minute" (the default for new jobs).
pub async fn default_first_run() -> String {
    utc_to_local_input(Utc::now() + Duration::minutes(1)).await
}
