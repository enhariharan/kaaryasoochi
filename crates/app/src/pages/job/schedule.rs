// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! The schedule editor: run once, every n seconds, or on a cron schedule built with the
//! Simple / Advanced / Raw tabs. The cron text in `cron` is always a valid, canonical
//! expression; every tab reads and writes it.

use chrono::{Duration, Utc};
use dioxus::prelude::*;
use kaaryasoochi_core::limits::REPEAT_N_MAX;
use kaaryasoochi_core::{describe, CronExpr, CronField, SimplePreset};

use crate::state::{effective_tz, fmt_datetime, local_input_to_utc, t};
use crate::Route;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SchedKind {
    Cron,
    Once,
    Seconds,
    /// An older repeat rule that has no exact cron equivalent; kept until the user picks another.
    Legacy,
}

impl SchedKind {
    fn value(self) -> &'static str {
        match self {
            Self::Cron => "cron",
            Self::Once => "once",
            Self::Seconds => "seconds",
            Self::Legacy => "legacy",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "once" => Self::Once,
            "seconds" => Self::Seconds,
            "legacy" => Self::Legacy,
            _ => Self::Cron,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Simple,
    Advanced,
    Raw,
}

/// Cron numbers Sunday as 0; show the week starting on Monday.
const DOW_ORDER: [u32; 7] = [1, 2, 3, 4, 5, 6, 0];

fn day_key(d: u32) -> &'static str {
    [
        "day.sun", "day.mon", "day.tue", "day.wed", "day.thu", "day.fri", "day.sat",
    ][d as usize % 7]
}

fn month_key(m: u32) -> &'static str {
    [
        "month.jan",
        "month.feb",
        "month.mar",
        "month.apr",
        "month.may",
        "month.jun",
        "month.jul",
        "month.aug",
        "month.sep",
        "month.oct",
        "month.nov",
        "month.dec",
    ][(m as usize + 11) % 12]
}

fn parse_hm(v: &str) -> Option<(u8, u8)> {
    let (h, m) = v.split_once(':')?;
    Some((
        h.parse().ok().filter(|h| *h < 24)?,
        m.parse().ok().filter(|m| *m < 60)?,
    ))
}

#[component]
pub fn ScheduleEditor(
    kind: Signal<SchedKind>,
    cron: Signal<String>,
    seconds: Signal<String>,
    first_run: Signal<String>,
    legacy: Option<String>,
) -> Element {
    let mut kind = kind;
    let mut seconds = seconds;
    let mut first_run = first_run;
    rsx! {
        div { class: "sched",
            select { value: kind().value(), onchange: move |e| kind.set(SchedKind::parse(&e.value())),
                option { value: "cron", selected: kind() == SchedKind::Cron, {t("sched.kind.cron")} }
                option { value: "once", selected: kind() == SchedKind::Once, {t("sched.kind.once")} }
                option { value: "seconds", selected: kind() == SchedKind::Seconds, {t("job.repeat.seconds")} }
                if legacy.is_some() {
                    option { value: "legacy", selected: kind() == SchedKind::Legacy, {t("sched.kind.legacy")} }
                }
            }
            match kind() {
                SchedKind::Cron => rsx! {
                    CronBuilder { cron }
                    label { {t("sched.start_from")}
                        input { r#type: "datetime-local", required: true, value: "{first_run}", oninput: move |e| first_run.set(e.value()) } }
                    Preview { cron, first_run }
                },
                SchedKind::Once => rsx! {
                    label { {t("job.first_run")} " *"
                        input { r#type: "datetime-local", required: true, value: "{first_run}", oninput: move |e| first_run.set(e.value()) } }
                },
                SchedKind::Seconds => rsx! {
                    label { {t("job.repeat.seconds")}
                        input { r#type: "number", min: 1, max: REPEAT_N_MAX as i64, value: "{seconds}", oninput: move |e| seconds.set(e.value()) } }
                    label { {t("sched.start_from")} " *"
                        input { r#type: "datetime-local", required: true, value: "{first_run}", oninput: move |e| first_run.set(e.value()) } }
                },
                SchedKind::Legacy => rsx! {
                    p { class: "muted", "{legacy.clone().unwrap_or_default()}" }
                },
            }
        }
    }
}

#[component]
fn CronBuilder(cron: Signal<String>) -> Element {
    let mut tab = use_signal(move || {
        let simple = cron
            .peek()
            .parse::<CronExpr>()
            .ok()
            .and_then(|x| SimplePreset::from_expr(&x));
        if simple.is_some() {
            Tab::Simple
        } else {
            Tab::Advanced
        }
    });
    let mut raw = use_signal(move || cron.peek().clone());
    let mut raw_err = use_signal(|| None::<String>);

    let tab_button = move |which: Tab, key: &'static str| {
        rsx! {
            button {
                r#type: "button", role: "tab", "aria-selected": tab() == which,
                class: if tab() == which { "tab active" } else { "tab" },
                onclick: move |_| {
                    if which == Tab::Raw {
                        raw.set(cron());
                        raw_err.set(None);
                    }
                    tab.set(which);
                },
                {t(key)}
            }
        }
    };

    rsx! {
        div { class: "cron-builder",
            div { class: "sched-tabs", role: "tablist",
                {tab_button(Tab::Simple, "sched.tab.simple")}
                {tab_button(Tab::Advanced, "sched.tab.advanced")}
                {tab_button(Tab::Raw, "sched.tab.raw")}
            }
            match tab() {
                Tab::Simple => rsx! { SimpleTab { cron } },
                Tab::Advanced => rsx! { AdvancedTab { cron } },
                Tab::Raw => rsx! { RawTab { cron, raw, raw_err } },
            }
        }
    }
}

// ---------------------------------------------------------------- Simple

#[component]
fn SimpleTab(cron: Signal<String>) -> Element {
    let current = use_memo(move || {
        cron.read()
            .parse::<CronExpr>()
            .ok()
            .and_then(|x| SimplePreset::from_expr(&x))
    });
    let set = move |p: SimplePreset| {
        let mut cron = cron;
        cron.set(p.to_expr().to_string());
    };
    let kind = match current() {
        Some(SimplePreset::EveryNMinutes(_)) => "minutes",
        Some(SimplePreset::EveryNHours { .. }) => "hours",
        Some(SimplePreset::Daily { .. }) => "daily",
        Some(SimplePreset::Weekly { .. }) => "weekly",
        Some(SimplePreset::Monthly { .. }) => "monthly",
        Some(SimplePreset::Yearly { .. }) => "yearly",
        None => "custom",
    };
    let pick = move |k: String| {
        set(match k.as_str() {
            "minutes" => SimplePreset::EveryNMinutes(15),
            "hours" => SimplePreset::EveryNHours { n: 1, minute: 0 },
            "daily" => SimplePreset::Daily { hour: 9, minute: 0 },
            "weekly" => SimplePreset::Weekly {
                days: 0b011_1110,
                hour: 9,
                minute: 0,
            },
            "monthly" => SimplePreset::Monthly {
                day: 1,
                hour: 9,
                minute: 0,
            },
            _ => SimplePreset::Yearly {
                month: 1,
                day: 1,
                hour: 9,
                minute: 0,
            },
        })
    };
    let time_input =
        move |hour: u8, minute: u8, rebuild: fn(SimplePreset, u8, u8) -> SimplePreset| {
            rsx! {
                label { {t("sched.at")}
                    input { r#type: "time", required: true, value: "{hour:02}:{minute:02}",
                        oninput: move |e| {
                            if let (Some((h, m)), Some(p)) = (parse_hm(&e.value()), current()) {
                                set(rebuild(p, h, m));
                            }
                        } }
                }
            }
        };
    let with_time: fn(SimplePreset, u8, u8) -> SimplePreset = |p, h, m| match p {
        SimplePreset::Daily { .. } => SimplePreset::Daily { hour: h, minute: m },
        SimplePreset::Weekly { days, .. } => SimplePreset::Weekly {
            days,
            hour: h,
            minute: m,
        },
        SimplePreset::Monthly { day, .. } => SimplePreset::Monthly {
            day,
            hour: h,
            minute: m,
        },
        SimplePreset::Yearly { month, day, .. } => SimplePreset::Yearly {
            month,
            day,
            hour: h,
            minute: m,
        },
        other => other,
    };

    rsx! {
        div { class: "simple",
            select { value: kind, onchange: move |e| pick(e.value()),
                if kind == "custom" { option { value: "custom", selected: true, disabled: true, "—" } }
                option { value: "minutes", selected: kind == "minutes", {t("sched.p.minutes")} }
                option { value: "hours", selected: kind == "hours", {t("sched.p.hours")} }
                option { value: "daily", selected: kind == "daily", {t("sched.p.daily")} }
                option { value: "weekly", selected: kind == "weekly", {t("sched.p.weekly")} }
                option { value: "monthly", selected: kind == "monthly", {t("sched.p.monthly")} }
                option { value: "yearly", selected: kind == "yearly", {t("sched.p.yearly")} }
            }
            match current() {
                Some(SimplePreset::EveryNMinutes(n)) => rsx! {
                    label { {t("sched.every")}
                        input { r#type: "number", min: 1, max: 59, value: "{n}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::EveryNMinutes(v.clamp(1, 59))) } }
                        {t("sched.minutes")} }
                },
                Some(SimplePreset::EveryNHours { n, minute }) => rsx! {
                    label { {t("sched.every")}
                        input { r#type: "number", min: 1, max: 23, value: "{n}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::EveryNHours { n: v.clamp(1, 23), minute }) } }
                        {t("sched.hours")} }
                    label { {t("sched.at_minute")}
                        input { r#type: "number", min: 0, max: 59, value: "{minute}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::EveryNHours { n, minute: v.clamp(0, 59) }) } } }
                },
                Some(SimplePreset::Daily { hour, minute }) => time_input(hour, minute, with_time),
                Some(SimplePreset::Weekly { days, hour, minute }) => rsx! {
                    div { class: "weekdays",
                        for d in DOW_ORDER {
                            label { key: "{d}", class: "check",
                                input { r#type: "checkbox", checked: days & (1 << d) != 0,
                                    onchange: move |e| {
                                        let next = if e.checked() { days | (1 << d) } else { days & !(1 << d) };
                                        if next != 0 { set(SimplePreset::Weekly { days: next, hour, minute }) }
                                    } }
                                {t(day_key(d))}
                            }
                        }
                        button { r#type: "button", class: "link", onclick: move |_| set(SimplePreset::Weekly { days: 0b011_1110, hour, minute }), {t("sched.weekdays")} }
                        button { r#type: "button", class: "link", onclick: move |_| set(SimplePreset::Weekly { days: 0b100_0001, hour, minute }), {t("sched.weekend")} }
                        button { r#type: "button", class: "link", onclick: move |_| set(SimplePreset::Weekly { days: 0b111_1111, hour, minute }), {t("sched.p.daily")} }
                    }
                    {time_input(hour, minute, with_time)}
                },
                Some(SimplePreset::Monthly { day, hour, minute }) => rsx! {
                    label { {t("cron.dom")}
                        input { r#type: "number", min: 1, max: 31, value: "{day}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::Monthly { day: v.clamp(1, 31), hour, minute }) } } }
                    {time_input(hour, minute, with_time)}
                    if day > 28 { p { class: "muted", {t("sched.dom_note")} } }
                },
                Some(SimplePreset::Yearly { month, day, hour, minute }) => rsx! {
                    label { {t("cron.month")}
                        select { value: "{month}",
                            onchange: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::Yearly { month: v, day, hour, minute }) },
                            for m in 1..=12u32 { option { key: "{m}", value: "{m}", selected: m as u8 == month, {t(month_key(m))} } }
                        } }
                    label { {t("cron.dom")}
                        input { r#type: "number", min: 1, max: 31, value: "{day}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u8>() { set(SimplePreset::Yearly { month, day: v.clamp(1, 31), hour, minute }) } } }
                    {time_input(hour, minute, with_time)}
                },
                None => rsx! { p { class: "muted", {t("sched.p.custom")} } },
            }
        }
    }
}

// ---------------------------------------------------------------- Advanced

#[component]
fn AdvancedTab(cron: Signal<String>) -> Element {
    let Some(expr) = cron.read().parse::<CronExpr>().ok() else {
        return rsx! { p { class: "error", "?" } };
    };
    rsx! {
        div { class: "advanced",
            for f in CronField::ALL {
                FieldEditor { key: "{f:?}", field: f, expr, cron }
            }
            if expr.day_fields_are_ored() { p { class: "muted", {t("cron.or_note")} } }
        }
    }
}

#[component]
fn FieldEditor(field: CronField, expr: CronExpr, cron: Signal<String>) -> Element {
    let (min, max) = field.range();
    let (label, default_step, default_one) = match field {
        CronField::Minute => ("cron.minute", 15, 0),
        CronField::Hour => ("cron.hour", 2, 9),
        CronField::DayOfMonth => ("cron.dom", 2, 1),
        CronField::Month => ("cron.month", 3, 1),
        CronField::DayOfWeek => ("cron.dow", 2, 1),
    };
    let every = expr.is_every(field);
    let step = expr.step(field);
    let selected = expr.values(field);
    let apply = move |x: CronExpr| {
        let mut cron = cron;
        cron.set(x.to_string());
    };
    let all: Vec<u32> = if field == CronField::DayOfWeek {
        DOW_ORDER.to_vec()
    } else {
        (min..=max).collect()
    };
    let stepped = move |n: u32| (min..=max).step_by(n.max(1) as usize).collect::<Vec<u32>>();
    let chip_label = move |v: u32| match field {
        CronField::Minute | CronField::Hour => format!("{v:02}"),
        CronField::DayOfMonth => v.to_string(),
        CronField::Month => t(month_key(v)).to_string(),
        CronField::DayOfWeek => t(day_key(v)).to_string(),
    };
    let all_values = all.clone();
    let sel_for_specific = selected.clone();

    rsx! {
        div { class: "cron-field",
            div { class: "cron-field-head",
                strong { {t(label)} }
                div { class: "seg",
                    button { r#type: "button", class: if every { "on" } else { "" },
                        onclick: move |_| apply(expr.with_values(field, &all_values, true)), {t("sched.every")} }
                    button { r#type: "button", class: if !every && step.is_none() { "on" } else { "" },
                        onclick: move |_| {
                            let vals = if every { vec![default_one] } else { sel_for_specific.clone() };
                            apply(expr.with_values(field, &vals, false));
                        }, {t("cron.mode.specific")} }
                    if field != CronField::DayOfWeek {
                        button { r#type: "button", class: if step.is_some() { "on" } else { "" },
                            onclick: move |_| apply(expr.with_values(field, &stepped(step.unwrap_or(default_step)), true)), {t("cron.mode.step")} }
                    }
                }
                if let Some(n) = step {
                    input { class: "step", r#type: "number", min: 2, max: (max - min) as i64, value: "{n}",
                        oninput: move |e| if let Ok(v) = e.value().parse::<u32>() {
                            apply(expr.with_values(field, &stepped(v.clamp(2, max - min)), true))
                        } }
                }
            }
            if !every {
                div { class: "chips",
                    for v in all {
                        button { key: "{v}", r#type: "button",
                            class: if selected.contains(&v) { "chip on" } else { "chip" },
                            onclick: {
                                let selected = selected.clone();
                                move |_| {
                                    let mut next = selected.clone();
                                    match next.iter().position(|x| *x == v) {
                                        Some(i) => { next.remove(i); }
                                        None => next.push(v),
                                    }
                                    // A field needs at least one value; `with_values` ignores empty.
                                    apply(expr.with_values(field, &next, false));
                                }
                            },
                            {chip_label(v)}
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- Raw

#[component]
fn RawTab(cron: Signal<String>, raw: Signal<String>, raw_err: Signal<Option<String>>) -> Element {
    let mut raw = raw;
    let mut raw_err = raw_err;
    let canonical = cron.read().clone();
    let differs = raw_err().is_none() && raw.read().trim() != canonical;
    rsx! {
        div { class: "raw",
            input { class: "mono", r#type: "text", spellcheck: false, autocomplete: "off",
                placeholder: "*/15 9-17 * * mon-fri", value: "{raw}",
                oninput: move |e| {
                    let v = e.value();
                    match v.parse::<CronExpr>() {
                        Ok(x) => {
                            let mut cron = cron;
                            cron.set(x.to_string());
                            raw_err.set(None);
                        }
                        Err(err) => raw_err.set(Some(err.to_string())),
                    }
                    raw.set(v);
                } }
            if let Some(e) = raw_err() { p { class: "error", role: "alert", "{e}" } }
            else if differs { p { class: "muted", {t("sched.raw.saved_as")} ": " code { "{canonical}" } } }
            p { class: "muted", {t("sched.raw.hint")} }
        }
    }
}

// ---------------------------------------------------------------- Preview

#[component]
fn Preview(cron: Signal<String>, first_run: Signal<String>) -> Element {
    let Some(x) = cron.read().parse::<CronExpr>().ok() else {
        return rsx! {};
    };
    let tz = effective_tz();
    let now = Utc::now();
    // Runs happen after the "start from" time, and never in the past.
    let after = local_input_to_utc(&first_run.read())
        .map(|s| {
            if s > now {
                s - Duration::seconds(1)
            } else {
                now
            }
        })
        .unwrap_or(now);
    let runs = x.next_n(after, tz, 5);
    rsx! {
        div { class: "preview",
            p { strong { {t("sched.meaning")} ": " } "{describe(&x)}" }
            p { strong { {t("sched.next")} ":" } }
            if runs.is_empty() { p { class: "muted", "—" } }
            ul { for r in runs { li { key: "{r.timestamp()}", bdi { dir: "ltr", "{fmt_datetime(r)}" } } } }
            p { class: "muted", "{tz.name()} · "
                Link { to: Route::Settings {}, {t("sched.tz.change")} } }
        }
    }
}
