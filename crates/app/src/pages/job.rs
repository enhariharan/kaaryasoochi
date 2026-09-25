// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use dioxus::prelude::*;
use kaaryasoochi_core::dto::{JobView, RunStatus};
use kaaryasoochi_core::limits::{
    CATEGORY_MAX, DESCRIPTION_MAX, REPEAT_N_MAX, SUMMARY_MAX, TITLE_MAX,
};
use kaaryasoochi_core::{JobInput, Repeat, Weekday};

use super::{clean_err, ErrorBanner};
use crate::state::{
    app_state, default_first_run, fmt_datetime, local_input_to_utc, t, token, utc_to_local_input,
};
use crate::{api, Route};

#[component]
pub fn JobNew() -> Element {
    rsx! { JobForm { id: None } }
}

#[component]
pub fn JobEdit(id: i32) -> Element {
    rsx! { JobForm { id: Some(id) } }
}

const KINDS: [(&str, &str); 8] = [
    ("none", "job.repeat.none"),
    ("seconds", "job.repeat.seconds"),
    ("minutes", "job.repeat.minutes"),
    ("days", "job.repeat.days"),
    ("weeks", "job.repeat.weeks"),
    ("months", "job.repeat.months"),
    ("weekday", "job.repeat.weekday"),
    ("day_of_month", "job.repeat.day_of_month"),
];
const WEEKDAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[component]
fn JobForm(id: Option<i32>) -> Element {
    let nav = use_navigator();
    let mut title = use_signal(String::new);
    let mut category = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut first_run = use_signal(String::new);
    let mut kind = use_signal(|| "none".to_string());
    let mut n = use_signal(|| "1".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let categories = use_resource(move || async move { api::list_categories(token()).await });

    // Populate the form once: from the stored job when editing, otherwise defaults.
    let loaded = use_resource(move || async move {
        match id {
            Some(id) => {
                let j: JobView = api::get_job(token(), id).await.map_err(clean_err)?;
                title.set(j.title);
                category.set(j.category);
                summary.set(j.summary);
                description.set(j.description);
                first_run.set(utc_to_local_input(j.first_run).await);
                if let Some((k, v)) = j.repeat.map(|r| r.encode()) {
                    kind.set(k.into());
                    n.set(v.to_string());
                }
            }
            None => first_run.set(default_first_run().await),
        }
        Ok::<_, String>(())
    });
    if let Some(Err(e)) = &*loaded.read() {
        return rsx! { p { class: "error", "{e}" } };
    }

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        spawn(async move {
            let res = async {
                let first = local_input_to_utc(&first_run())
                    .await
                    .ok_or("Enter a valid date and time")?;
                let repeat = build_repeat(&kind(), n().trim().parse().unwrap_or(0));
                let input = JobInput {
                    title: title(),
                    category: category(),
                    summary: summary(),
                    description: description(),
                    first_run: first,
                    repeat: repeat?,
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

    let has_n = matches!(
        kind().as_str(),
        "seconds" | "minutes" | "days" | "weeks" | "months" | "day_of_month"
    );
    let n_max = if kind() == "day_of_month" {
        31
    } else {
        REPEAT_N_MAX
    };

    rsx! {
        section { class: "card",
            h1 { if id.is_some() { "{title}" } else { {t("job.add")} } }
            form { onsubmit: submit,
                label { {t("job.title")} " *"
                    input { r#type: "text", required: true, maxlength: TITLE_MAX as i64, value: "{title}", oninput: move |e| title.set(e.value()) } }
                label { {t("job.category")}
                    // A datalist gives type-to-search over existing categories while still accepting a new name.
                    input { r#type: "text", list: "categories", maxlength: CATEGORY_MAX as i64, placeholder: "Default",
                        value: "{category}", oninput: move |e| category.set(e.value()) }
                    datalist { id: "categories",
                        if let Some(Ok(cats)) = &*categories.read() { for c in cats { option { key: "{c.id}", value: "{c.name}" } } }
                    } }
                label { {t("job.summary")}
                    input { r#type: "text", maxlength: SUMMARY_MAX as i64, value: "{summary}", oninput: move |e| summary.set(e.value()) } }
                label { {t("job.description")}
                    textarea { rows: 5, maxlength: DESCRIPTION_MAX as i64, value: "{description}", oninput: move |e| description.set(e.value()) } }
                label { {t("job.first_run")} " *"
                    input { r#type: "datetime-local", required: true, value: "{first_run}", oninput: move |e| first_run.set(e.value()) } }
                fieldset { legend { {t("job.repeat")} }
                    select { value: "{kind}", onchange: move |e| kind.set(e.value()),
                        for (k, key) in KINDS { option { key: "{k}", value: k, selected: kind() == k, "{t(key)}" } }
                    }
                    if has_n { input { r#type: "number", min: 1, max: n_max as i64, value: "{n}", oninput: move |e| n.set(e.value()) } }
                    if kind() == "weekday" {
                        select { value: "{n}", onchange: move |e| n.set(e.value()),
                            for (i, d) in WEEKDAY_NAMES.iter().enumerate() { option { key: "{i}", value: "{i}", selected: n() == i.to_string(), "{d}" } }
                        }
                    }
                }
                ErrorBanner { msg: error }
                div { class: "actions",
                    button { class: "primary", r#type: "submit", disabled: busy(), {t("common.save")} }
                    Link { to: Route::Home {}, class: "button", {t("common.cancel")} }
                    if let Some(id) = id {
                        button { class: "danger", r#type: "button", onclick: move |_| {
                            spawn(async move {
                                match api::delete_job(token(), id).await { Ok(()) => { nav.push(Route::Home {}); } Err(e) => error.set(Some(clean_err(e))) }
                            });
                        }, {t("common.delete")} }
                    }
                }
            }
        }
        if let Some(id) = id { History { job_id: id } }
    }
}

/// Weekday selection is stored 0..=6 in `n`; other kinds use it as the interval.
fn build_repeat(kind: &str, n: u32) -> Result<Option<Repeat>, String> {
    if kind == "none" {
        return Ok(None);
    }
    let r = if kind == "weekday" {
        Repeat::Weekday(*Weekday::ALL.get(n as usize).unwrap_or(&Weekday::Mon))
    } else {
        Repeat::decode(kind, n).map_err(|e| e.to_string())?
    };
    r.validate().map_err(|e| e.to_string())?;
    Ok(Some(r))
}

#[component]
fn History(job_id: i32) -> Element {
    let page_size = app_state().settings.read().page_size;
    let mut page = use_signal(|| 1u32);
    let runs =
        use_resource(
            move || async move { api::job_history(token(), job_id, page(), page_size).await },
        );

    rsx! {
        section { class: "card",
            h2 { {t("job.history")} }
            match &*runs.read() {
                None => rsx! { p { class: "muted", "…" } },
                Some(Err(e)) => rsx! { p { class: "error", "{e}" } },
                Some(Ok(p)) => rsx! {
                    if p.items.is_empty() { p { class: "muted", "—" } } else {
                        table {
                            thead { tr { th { {t("job.first_run")} } th { {t("job.status")} } th { "" } } }
                            tbody { for r in &p.items {
                                tr { key: "{r.id}",
                                    td { "{fmt_datetime(r.started_at)}" }
                                    td { span { class: "status {r.status.as_str()}", "{status_label(r.status)}" } }
                                    td { class: "muted", "{r.message}" }
                                }
                            } }
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

fn status_label(s: RunStatus) -> &'static str {
    match s {
        RunStatus::Running => "Running",
        RunStatus::Success => "Success",
        RunStatus::Failed => "Failed",
    }
}
