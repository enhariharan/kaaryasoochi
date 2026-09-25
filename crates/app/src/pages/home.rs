// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use dioxus::prelude::*;
use kaaryasoochi_core::dto::JobView;
use kaaryasoochi_core::{Layout, TabOrientation, DEFAULT_CATEGORY};

use crate::state::{app_state, fmt_datetime, t, token};
use crate::{api, Route};

#[component]
pub fn Home() -> Element {
    let s = app_state();
    let categories = use_resource(move || async move { api::list_categories(token()).await });
    let jobs = use_resource(move || async move { api::list_jobs(token()).await });
    let mut selected = use_signal(|| DEFAULT_CATEGORY.to_string());

    let cfg = s.settings.read().clone();
    let (Some(Ok(cats)), Some(Ok(all))) = (&*categories.read(), &*jobs.read()) else {
        return match (&*categories.read(), &*jobs.read()) {
            (Some(Err(e)), _) | (_, Some(Err(e))) => rsx! { p { class: "error", "{e}" } },
            _ => rsx! { p { class: "muted center", "…" } },
        };
    };
    let visible: Vec<&JobView> = all.iter().filter(|j| j.category == selected()).collect();
    let orientation = match cfg.tab_orientation {
        TabOrientation::Horizontal => "tabs-h",
        TabOrientation::Vertical => "tabs-v",
    };
    let layout = match cfg.job_layout {
        Layout::Grid => "jobs grid",
        Layout::List => "jobs list",
    };

    rsx! {
        div { class: "home {orientation}",
            div { class: "tabs", role: "tablist",
                for c in cats.iter() {
                    button {
                        key: "{c.id}", role: "tab", class: if c.name == selected() { "tab active" } else { "tab" },
                        "aria-selected": c.name == selected(),
                        onclick: { let n = c.name.clone(); move |_| selected.set(n.clone()) },
                        "{c.name}"
                        span { class: "count", "{all.iter().filter(|j| j.category == c.name).count()}" }
                    }
                }
            }
            div { class: "tab-panel", role: "tabpanel",
                if visible.is_empty() { p { class: "muted", {t("home.empty")} } }
                div { class: "{layout}",
                    for j in visible {
                        Link { key: "{j.id}", to: Route::JobEdit { id: j.id }, class: "job-card",
                            h3 { "{j.title}" }
                            if !j.summary.is_empty() { p { "{j.summary}" } }
                            if let Some(n) = j.next_run { small { class: "muted", "{fmt_datetime(n)}" } }
                        }
                    }
                }
            }
        }
        Link { to: Route::JobNew {}, class: "fab", title: t("job.add"), "aria-label": t("job.add"), "+" }
    }
}
