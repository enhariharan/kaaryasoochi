// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Job details page: Basics, What to run, Schedule, Options, then a folded run history.

mod history;
mod picker;
mod schedule;

use dioxus::prelude::*;
use kaaryasoochi_core::dto::{JobView, RunStatus};
use kaaryasoochi_core::limits::{
    CATEGORY_MAX, COMMAND_MAX, DESCRIPTION_MAX, PATH_MAX, RETRY_COUNT_DEFAULT, RETRY_COUNT_MAX,
    RETRY_DELAY_DEFAULT_SECS, RETRY_DELAY_MAX_SECS, SCRIPT_ARGS_MAX, SUMMARY_MAX,
    TIMEOUT_DEFAULT_SECS, TIMEOUT_MAX_SECS, TITLE_MAX,
};
use kaaryasoochi_core::timezone::{self, Tz};
use kaaryasoochi_core::{JobInput, Repeat, RunMode};

use super::{clean_err, ErrorBanner};
use crate::state::{
    default_first_run, effective_tz, local_input_to_utc, sleep_ms, t, token, utc_to_local_input,
};
use crate::{api, Route};
use history::History;
use picker::FilePicker;
use schedule::{SchedKind, ScheduleEditor};

#[component]
pub fn JobNew() -> Element {
    rsx! { JobForm { id: None } }
}

#[component]
pub fn JobEdit(id: i32) -> Element {
    rsx! { JobForm { id: Some(id) } }
}

/// English label for an older repeat rule that has no exact cron equivalent.
fn legacy_label(r: Repeat) -> String {
    match r {
        Repeat::EveryMinutes(n) => format!("Every {n} minutes"),
        Repeat::EveryDays(n) => format!("Every {n} days"),
        Repeat::EveryWeeks(n) => format!("Every {n} weeks"),
        Repeat::EveryMonths(n) => format!("Every {n} months"),
        Repeat::DayOfMonth(d) => format!("On day {d} of every month (last day in shorter months)"),
        other => format!("{other:?}"),
    }
}

#[component]
fn JobForm(id: Option<i32>) -> Element {
    let nav = use_navigator();
    // Basics
    let mut title = use_signal(String::new);
    let mut category = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut description = use_signal(String::new);
    // What to run
    let mut run_mode = use_signal(|| RunMode::Command);
    let mut command = use_signal(String::new);
    let mut script_path = use_signal(String::new);
    let mut script_args = use_signal(String::new);
    let mut working_dir = use_signal(String::new);
    // Schedule
    let mut sched_kind = use_signal(|| SchedKind::Cron);
    let mut cron = use_signal(|| "0 9 * * *".to_string());
    let mut seconds = use_signal(|| "30".to_string());
    let mut first_run = use_signal(String::new);
    let mut legacy = use_signal(|| None::<Repeat>);
    // Options
    let mut timeout = use_signal(|| TIMEOUT_DEFAULT_SECS.to_string());
    let mut notify = use_signal(|| true);
    let mut retry_on = use_signal(|| false);
    let mut retry_count = use_signal(|| RETRY_COUNT_DEFAULT.to_string());
    let mut retry_delay = use_signal(|| RETRY_DELAY_DEFAULT_SECS.to_string());
    // Page state
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut picker_open = use_signal(|| false);
    let mut hist_open = use_signal(|| false); // folded by default
    let mut refresh = use_signal(|| 0u32);

    let categories = use_resource(move || async move { api::list_categories(token()).await });
    // Only administrators may attach a command to a job.
    let me = use_resource(move || async move { api::me(token()).await });
    let is_admin = matches!(&*me.read(), Some(Ok(u)) if u.is_admin);

    // Populate the form once: from the stored job when editing, otherwise defaults.
    let loaded = use_resource(move || async move {
        match id {
            Some(id) => {
                let j: JobView = api::get_job(token(), id).await.map_err(clean_err)?;
                title.set(j.title);
                category.set(j.category);
                summary.set(j.summary);
                description.set(j.description);
                run_mode.set(j.run_mode);
                command.set(j.command);
                script_path.set(j.script_path);
                script_args.set(j.script_args);
                working_dir.set(j.working_dir);
                timeout.set(j.timeout_secs.to_string());
                notify.set(j.notify_on_failure);
                retry_on.set(j.retry_on_failure);
                retry_count.set(j.retry_count.to_string());
                retry_delay.set(j.retry_delay_secs.to_string());
                first_run.set(utc_to_local_input(j.first_run));
                let tz: Tz = timezone::parse(&j.timezone).unwrap_or(Tz::UTC);
                match j.repeat {
                    None => sched_kind.set(SchedKind::Once),
                    Some(Repeat::EverySeconds(n)) => {
                        seconds.set(n.to_string());
                        sched_kind.set(SchedKind::Seconds);
                    }
                    Some(r) => match r.to_cron(j.first_run, tz) {
                        Some(x) => {
                            cron.set(x.to_string());
                            sched_kind.set(SchedKind::Cron);
                        }
                        None => {
                            legacy.set(Some(r));
                            sched_kind.set(SchedKind::Legacy);
                        }
                    },
                }
            }
            None => first_run.set(default_first_run()),
        }
        Ok::<_, String>(())
    });
    match &*loaded.read() {
        Some(Err(e)) => return rsx! { p { class: "error", "{e}" } },
        None => return rsx! { p { class: "muted center", "…" } },
        Some(Ok(())) => {}
    }

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        spawn(async move {
            let res = async {
                let first = local_input_to_utc(&first_run()).ok_or(t("job.invalid_datetime"))?;
                let repeat = match sched_kind() {
                    SchedKind::Cron => Some(Repeat::Cron(
                        cron()
                            .parse()
                            .map_err(|e: kaaryasoochi_core::CronError| e.to_string())?,
                    )),
                    SchedKind::Once => None,
                    SchedKind::Seconds => {
                        let n = seconds().trim().parse().unwrap_or(0);
                        let r = Repeat::EverySeconds(n);
                        r.validate().map_err(|e| e.to_string())?;
                        Some(r)
                    }
                    SchedKind::Legacy => legacy(),
                };
                let input = JobInput {
                    title: title(),
                    category: category(),
                    summary: summary(),
                    description: description(),
                    first_run: first,
                    repeat,
                    run_mode: run_mode(),
                    command: command(),
                    script_path: script_path(),
                    script_args: script_args(),
                    working_dir: working_dir(),
                    timeout_secs: timeout().trim().parse().unwrap_or(0),
                    notify_on_failure: notify(),
                    retry_on_failure: retry_on(),
                    retry_count: retry_count().trim().parse().unwrap_or(0),
                    retry_delay_secs: retry_delay().trim().parse().unwrap_or(0),
                };
                api::save_job(token(), id, input).await.map_err(clean_err)
            }
            .await;
            match res {
                Ok(_) => {
                    nav.push(Route::Home {});
                }
                Err(m) => error.set(Some(m)),
            }
            busy.set(false);
        });
    };

    let run_now = move |_| {
        let Some(id) = id else { return };
        error.set(None);
        spawn(async move {
            match api::run_job_now(token(), id).await {
                Ok(()) => {
                    info.set(Some(t("job.run_now.started").to_string()));
                    hist_open.set(true);
                    refresh += 1;
                    // Follow the run (and any automatic retries) until done, or ~3 minutes pass.
                    for _ in 0..90 {
                        sleep_ms(2000).await;
                        refresh += 1;
                        match api::job_run_summary(token(), id).await {
                            // Keep going while a run is in progress or a retry is waiting.
                            Ok(s)
                                if s.retry_at.is_some()
                                    || s.last.as_ref().map(|r| r.status)
                                        == Some(RunStatus::Running) =>
                            {
                                continue
                            }
                            _ => break,
                        }
                    }
                }
                Err(e) => error.set(Some(clean_err(e))),
            }
        });
    };

    let legacy_text = legacy().map(legacy_label);

    rsx! {
        form { class: "job-page", onsubmit: submit,
            section { class: "card",
                h1 { if id.is_some() { "{title}" } else { {t("job.add")} } }
                label { {t("job.title")} " *"
                    input { r#type: "text", dir: "auto", required: true, maxlength: TITLE_MAX as i64, value: "{title}", oninput: move |e| title.set(e.value()) } }
                label { {t("job.category")}
                    // A datalist gives type-to-search over existing categories while still accepting a new name.
                    input { r#type: "text", dir: "auto", list: "categories", maxlength: CATEGORY_MAX as i64, placeholder: "Default",
                        value: "{category}", oninput: move |e| category.set(e.value()) }
                    datalist { id: "categories",
                        if let Some(Ok(cats)) = &*categories.read() { for c in cats { option { key: "{c.id}", value: "{c.name}" } } }
                    } }
                label { {t("job.summary")}
                    input { r#type: "text", dir: "auto", maxlength: SUMMARY_MAX as i64, value: "{summary}", oninput: move |e| summary.set(e.value()) } }
                label { {t("job.description")}
                    textarea { rows: 4, dir: "auto", maxlength: DESCRIPTION_MAX as i64, value: "{description}", oninput: move |e| description.set(e.value()) } }
            }

            section { class: "card",
                h2 { {t("job.section.what")} }
                if is_admin {
                    div { class: "seg wide",
                        button { r#type: "button", class: if run_mode() == RunMode::Command { "on" } else { "" },
                            onclick: move |_| run_mode.set(RunMode::Command), {t("job.mode.command")} }
                        button { r#type: "button", class: if run_mode() == RunMode::Script { "on" } else { "" },
                            onclick: move |_| run_mode.set(RunMode::Script), {t("job.mode.script")} }
                    }
                    if run_mode() == RunMode::Command {
                        label { {t("job.command")}
                            textarea { rows: 3, class: "mono", maxlength: COMMAND_MAX as i64, placeholder: "/path/to/script.sh --flag",
                                value: "{command}", oninput: move |e| command.set(e.value()) } }
                    } else {
                        label { {t("job.script.path")}
                            div { class: "path-row",
                                input { class: "mono", r#type: "text", maxlength: PATH_MAX as i64, placeholder: "/home/you/scripts/backup.sh",
                                    value: "{script_path}", oninput: move |e| script_path.set(e.value()) }
                                button { r#type: "button", onclick: move |_| picker_open.set(true), {t("job.script.browse")} }
                            } }
                        ScriptStatus { path: script_path }
                        label { {t("job.script.args")}
                            input { class: "mono", r#type: "text", maxlength: SCRIPT_ARGS_MAX as i64, placeholder: "--verbose \"two words\"",
                                value: "{script_args}", oninput: move |e| script_args.set(e.value()) } }
                    }
                    label { {t("job.working_dir")}
                        input { class: "mono", r#type: "text", maxlength: PATH_MAX as i64, placeholder: "~",
                            value: "{working_dir}", oninput: move |e| working_dir.set(e.value()) }
                        small { class: "muted", {t("job.working_dir.hint")} } }
                } else {
                    p { class: "muted", {t("job.admin_only")} }
                }
            }
            if picker_open() {
                FilePicker { open: picker_open, start: script_path(), on_pick: move |p: String| script_path.set(p) }
            }

            section { class: "card",
                h2 { {t("job.section.schedule")} }
                p { class: "muted", "{effective_tz().name()}" }
                ScheduleEditor { kind: sched_kind, cron, seconds, first_run, legacy: legacy_text }
            }

            section { class: "card",
                h2 { {t("job.section.options")} }
                label { {t("job.timeout")}
                    input { r#type: "number", min: 1, max: TIMEOUT_MAX_SECS as i64, value: "{timeout}", oninput: move |e| timeout.set(e.value()) } }
                label { class: "check",
                    input { r#type: "checkbox", checked: notify(), onchange: move |e| notify.set(e.checked()) }
                    {t("job.notify")} }
                label { class: "check",
                    input { r#type: "checkbox", checked: retry_on(), onchange: move |e| retry_on.set(e.checked()) }
                    {t("job.retry")} }
                if retry_on() {
                    div { class: "retry-opts",
                        label { {t("job.retry.count")}
                            input { r#type: "number", min: 1, max: RETRY_COUNT_MAX as i64, value: "{retry_count}",
                                oninput: move |e| retry_count.set(e.value()) } }
                        label { {t("job.retry.delay")}
                            input { r#type: "number", min: 1, max: RETRY_DELAY_MAX_SECS as i64, value: "{retry_delay}",
                                oninput: move |e| retry_delay.set(e.value()) } }
                    }
                }
            }

            ErrorBanner { msg: error }
            if let Some(m) = info() { p { class: "muted", role: "status", "{m}" } }
            div { class: "actions sticky",
                button { class: "primary", r#type: "submit", disabled: busy(), {t("common.save")} }
                Link { to: Route::Home {}, class: "button", {t("common.cancel")} }
                if let Some(id) = id {
                    button { r#type: "button", onclick: run_now, {t("job.run_now")} }
                    button { class: "danger", r#type: "button", onclick: move |_| {
                        spawn(async move {
                            match api::delete_job(token(), id).await { Ok(()) => { nav.push(Route::Home {}); } Err(e) => error.set(Some(clean_err(e))) }
                        });
                    }, {t("common.delete")} }
                }
            }
        }
        if let Some(id) = id { History { job_id: id, refresh, open: hist_open } }
    }
}

/// Shows whether the typed script path exists and is executable, with a one-click fix.
#[component]
fn ScriptStatus(path: Signal<String>) -> Element {
    let mut recheck = use_signal(|| 0u32);
    let mut err = use_signal(|| None::<String>);
    let info = use_resource(move || async move {
        recheck();
        let p = path();
        if p.trim().is_empty() {
            return None;
        }
        sleep_ms(300).await; // debounce: a newer keystroke cancels this run
        api::inspect_path(token(), p).await.ok()
    });
    let Some(Some(i)) = &*info.read() else {
        return rsx! {};
    };
    let p = path();
    rsx! {
        if !i.exists || !i.is_file {
            p { class: "error", {t("job.script.missing")} }
        } else if !i.is_executable {
            p { class: "warn", {t("job.script.not_exec")} " "
                button { r#type: "button", class: "link", onclick: move |_| {
                    let p = p.clone();
                    spawn(async move {
                        match api::make_executable(token(), p).await {
                            Ok(_) => { err.set(None); recheck += 1; }
                            Err(e) => err.set(Some(clean_err(e))),
                        }
                    });
                }, {t("job.script.make_exec")} } }
        } else {
            p { class: "ok", "✓ " {t("job.script.ok")} }
        }
        if let Some(e) = err() { p { class: "error", "{e}" } }
    }
}
