// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! App-wide client state, provided as context by `App` and read by every page.

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use dioxus::document::eval;
use dioxus::prelude::*;
use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::timezone::{self, Tz};
use kaaryasoochi_core::{tr, DateFormat, TimeFormat};

const TOKEN_KEY: &str = "kaaryasoochi.token";
/// Value format of `<input type="datetime-local">` (seconds are optional).
const INPUT_FMT: &str = "%Y-%m-%dT%H:%M";

#[derive(Clone, Copy)]
pub struct AppState {
    pub token: Signal<Option<String>>,
    pub settings: Signal<UserSettings>,
    /// IANA name of the device's zone, used while the time-zone setting is "system".
    pub system_tz: Signal<String>,
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

/// The zone the user wants times shown and entered in.
pub fn effective_tz() -> Tz {
    let s = app_state();
    let setting = s.settings.read().timezone.clone();
    let name = if setting == timezone::SYSTEM {
        s.system_tz.read().clone()
    } else {
        setting
    };
    timezone::parse(&name).unwrap_or(Tz::UTC)
}

/// Date and time in the user's chosen (or the device's) formats and time zone.
pub fn fmt_datetime(dt: DateTime<Utc>) -> String {
    let s = app_state();
    let local = dt.with_timezone(&effective_tz()).naive_local();
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

/// `datetime-local` value (wall-clock time in the user's zone) -> UTC.
pub fn local_input_to_utc(v: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(v, INPUT_FMT)
        .or_else(|_| NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    effective_tz()
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
}

/// UTC -> `datetime-local` value in the user's zone.
pub fn utc_to_local_input(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&effective_tz())
        .format(INPUT_FMT)
        .to_string()
}

/// `datetime-local` value for "now + 1 minute" (the default for new jobs).
pub fn default_first_run() -> String {
    utc_to_local_input(Utc::now() + Duration::minutes(1))
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
    pub timezone: String,
    pub date: DateFormat,
    pub time: TimeFormat,
}

pub async fn detect_locale() -> Locale {
    let mut e = eval(
        r#"
        const parts = new Intl.DateTimeFormat(undefined).formatToParts(new Date(2000, 10, 22));
        const order = parts.filter(p => ['day','month','year'].includes(p.type)).map(p => p.type[0]).join('');
        const h12 = new Intl.DateTimeFormat(undefined, {hour: 'numeric'}).resolvedOptions().hour12 === true;
        dioxus.send([Intl.DateTimeFormat().resolvedOptions().timeZone, order, h12]);
    "#,
    );
    match e.recv::<(String, String, bool)>().await {
        Ok((timezone, order, h12)) => Locale {
            timezone,
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
            timezone: "UTC".into(),
            date: DateFormat::Iso,
            time: TimeFormat::H24,
        },
    }
}

/// Async sleep that works in the browser and in desktop webviews.
pub async fn sleep_ms(ms: u32) {
    let mut e = eval(&format!("setTimeout(() => dioxus.send(1), {ms})"));
    let _ = e.recv::<i32>().await;
}
