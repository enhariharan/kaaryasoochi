// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Run history: folded by default, but the header always shows the last run and the run count.
//! The list itself is only fetched once the section is opened.

use dioxus::prelude::*;
use kaaryasoochi_core::dto::{RunStatus, RunView};

use crate::api;
use crate::state::{app_state, fmt_datetime, t, token};

fn status_label(s: RunStatus) -> &'static str {
    match s {
        RunStatus::Running => t("run.status.running"),
        RunStatus::Success => t("run.status.success"),
        RunStatus::Failed => t("run.status.failed"),
    }
}

fn status_icon(s: RunStatus) -> &'static str {
    match s {
        RunStatus::Running => "…",
        RunStatus::Success => "✓",
        RunStatus::Failed => "✗",
    }
}

/// First non-empty line, shortened for the collapsed output cell.
fn first_line(msg: &str) -> String {
    let line = msg.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let short: String = line.chars().take(60).collect();
    if short.len() < line.len() || msg.lines().count() > 1 {
        format!("{short}…")
    } else {
        short
    }
}

#[component]
pub fn History(job_id: i32, refresh: Signal<u32>, open: Signal<bool>) -> Element {
    let mut open = open;
    let page_size = app_state().settings.read().page_size;
    let mut page = use_signal(|| 1u32);
    let summary = use_resource(move || async move {
        refresh(); // re-run when a run is started or finishes
        api::job_run_summary(token(), job_id).await
    });
    let runs = use_resource(move || async move {
        refresh();
        if !open() {
            return Ok(None);
        }
        api::job_history(token(), job_id, page(), page_size)
            .await
            .map(Some)
    });

    rsx! {
        section { class: "card history",
            button { class: "history-toggle", r#type: "button", "aria-expanded": open(),
                onclick: move |_| open.set(!open()),
                span { class: if open() { "chev" } else { "chev closed" }, if open() { "▾" } else { "▸" } }
                strong { {t("job.history")} }
                match &*summary.read() {
                    Some(Ok(s)) => rsx! {
                        span { class: "summary",
                            match &s.last {
                                Some(r) => rsx! {
                                    span { class: "status {r.status.as_str()}", title: status_label(r.status),
                                        "{status_icon(r.status)} {status_label(r.status)}" }
                                    span { class: "muted", " · " bdi { dir: "ltr", "{fmt_datetime(r.started_at)}" } }
                                    if let Some(c) = r.exit_code { span { class: "muted", " · " {t("run.exit_code")} " {c}" } }
                                },
                                None => rsx! { span { class: "muted", {t("hist.none")} } },
                            }
                            span { class: "muted", " · " {t("hist.runs")} " {s.total}" }
                            if let Some(at) = s.retry_at {
                                span { class: "muted", " · ↻ " {t("hist.retry_at")} " " bdi { dir: "ltr", "{fmt_datetime(at)}" } }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
            if open() {
                match &*runs.read() {
                    None => rsx! { p { class: "muted", "…" } },
                    Some(Err(e)) => rsx! { p { class: "error", "{e}" } },
                    Some(Ok(None)) => rsx! {},
                    Some(Ok(Some(p))) => rsx! {
                        if p.items.is_empty() { p { class: "muted", {t("hist.none")} } } else {
                            table {
                                thead { tr { th { {t("hist.started")} } th { {t("job.status")} } th { {t("run.exit_code")} } th { {t("run.output")} } } }
                                tbody { for r in p.items.iter() { RunRow { key: "{r.id}", run: r.clone() } } }
                            }
                        }
                        div { class: "pager",
                            button { disabled: p.page <= 1, onclick: move |_| page -= 1, {t("common.prev")} }
                            span { "{p.page} / {p.total_pages()}" }
                            button { disabled: p.page >= p.total_pages(), onclick: move |_| page += 1, {t("common.next")} }
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn RunRow(run: RunView) -> Element {
    rsx! {
        tr {
            td {
                bdi { dir: "ltr", "{fmt_datetime(run.started_at)}" }
                if run.attempt > 1 { span { class: "muted", " · " {t("hist.attempt")} " {run.attempt}" } }
            }
            td { span { class: "status {run.status.as_str()}", "{status_icon(run.status)} {status_label(run.status)}" } }
            td { class: "muted", if let Some(c) = run.exit_code { "{c}" } else { "—" } }
            td {
                if run.message.is_empty() { span { class: "muted", "—" } } else {
                    details { summary { class: "muted", "{first_line(&run.message)}" } pre { class: "output", "{run.message}" } }
                }
            }
        }
    }
}
