// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! In-app file browser over the *server's* filesystem (admin only). Works the same in the
//! browser and the desktop app, and only shows paths the server can actually run.

use dioxus::prelude::*;

use crate::api;
use crate::pages::clean_err;
use crate::state::{t, token};

/// Directory part of a file path ("" when there is none, which lists the home directory).
fn start_dir(path: &str) -> String {
    match path.trim().rsplit_once('/') {
        Some(("", _)) => "/".into(),
        Some((dir, _)) => dir.into(),
        None => String::new(),
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir == "/" {
        format!("/{name}")
    } else {
        format!("{dir}/{name}")
    }
}

#[component]
pub fn FilePicker(open: Signal<bool>, start: String, on_pick: EventHandler<String>) -> Element {
    let mut open = open;
    let mut dir = use_signal(move || start_dir(&start));
    let mut location = use_signal(String::new);
    let mut show_hidden = use_signal(|| false);
    let mut selected = use_signal(|| None::<String>);
    let listing =
        use_resource(move || async move { api::browse_dir(token(), dir(), show_hidden()).await });

    // Keep the location box showing the directory actually listed (after canonicalisation).
    use_effect(move || {
        if let Some(Ok(l)) = &*listing.read() {
            location.set(l.path.clone());
        }
    });

    let mut pick = move |path: String| {
        on_pick.call(path);
        open.set(false);
    };

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| open.set(false),
            div { class: "modal", role: "dialog", "aria-modal": true, onclick: move |e| e.stop_propagation(),
                h2 { {t("picker.title")} }
                form { class: "picker-bar",
                    onsubmit: move |e| { e.prevent_default(); dir.set(location()); selected.set(None); },
                    button { r#type: "button",
                        disabled: !matches!(&*listing.read(), Some(Ok(l)) if l.parent.is_some()),
                        onclick: move |_| {
                            if let Some(Ok(l)) = &*listing.read() {
                                if let Some(p) = l.parent.clone() { dir.set(p); selected.set(None); }
                            }
                        }, "↑ " {t("picker.up")} }
                    input { class: "mono", r#type: "text", "aria-label": t("picker.location"), value: "{location}",
                        oninput: move |e| location.set(e.value()) }
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: show_hidden(), onchange: move |e| show_hidden.set(e.checked()) }
                    {t("picker.hidden")} }
                div { class: "picker-list",
                    match &*listing.read() {
                        None => rsx! { p { class: "muted", "…" } },
                        Some(Err(e)) => rsx! { p { class: "error", "{clean_err(e.clone())}" } },
                        Some(Ok(l)) => rsx! {
                            if l.entries.is_empty() { p { class: "muted", {t("picker.empty")} } }
                            for e in l.entries.iter() {
                                {
                                    let full = join(&l.path, &e.name);
                                    let (is_dir, chosen) = (e.is_dir, selected().as_deref() == Some(full.as_str()));
                                    let icon = if e.is_dir { "📁" } else if e.is_executable { "▶" } else { "📄" };
                                    let class = if chosen { "entry sel" } else { "entry" };
                                    rsx! {
                                        button { key: "{e.name}", r#type: "button", class,
                                            onclick: { let full = full.clone(); move |_| {
                                                if is_dir { dir.set(full.clone()); selected.set(None); } else { selected.set(Some(full.clone())); }
                                            } },
                                            ondoubleclick: { let full = full.clone(); move |_| if !is_dir { pick(full.clone()) } },
                                            span { class: "icon", "{icon}" }
                                            span { class: "name", "{e.name}" }
                                        }
                                    }
                                }
                            }
                            if l.truncated { p { class: "muted", {t("picker.truncated")} } }
                        },
                    }
                }
                if let Some(s) = selected() { p { class: "mono muted", "{s}" } }
                div { class: "actions",
                    button { class: "primary", r#type: "button", disabled: selected().is_none(),
                        onclick: move |_| if let Some(s) = selected() { pick(s) }, {t("picker.select")} }
                    button { r#type: "button", onclick: move |_| open.set(false), {t("common.cancel")} }
                }
            }
        }
    }
}
