// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

mod auth;
mod home;
mod job;
mod settings;

pub use auth::{Login, Register};
pub use home::Home;
pub use job::{JobEdit, JobNew};
pub use settings::Settings;

use dioxus::prelude::*;

use crate::state::t;

/// Inline error banner; renders nothing when there is no message.
#[component]
pub fn ErrorBanner(msg: ReadSignal<Option<String>>) -> Element {
    match msg() {
        Some(m) => rsx! { p { class: "error", role: "alert", "{m}" } },
        None => rsx! {},
    }
}

/// Password input with a show/hide (eye) toggle. Reads and writes `value`; the toggle only
/// switches the input between `password` and `text`, so autofill and password managers still work.
#[component]
pub fn PasswordField(
    label: &'static str,
    value: Signal<String>,
    autocomplete: &'static str,
    #[props(default = 0)] minlength: i64,
    #[props(default = 0)] maxlength: i64,
) -> Element {
    let mut value = value;
    let mut shown = use_signal(|| false);
    let hint = if shown() {
        t("auth.hide_password")
    } else {
        t("auth.show_password")
    };
    rsx! {
        label { {t(label)}
            div { class: "password-field",
                input {
                    r#type: if shown() { "text" } else { "password" },
                    autocomplete, required: true, spellcheck: false,
                    minlength: if minlength > 0 { Some(minlength) } else { None },
                    maxlength: if maxlength > 0 { Some(maxlength) } else { None },
                    value: "{value}", oninput: move |e| value.set(e.value()),
                }
                button { class: "eye", r#type: "button", title: hint, "aria-label": hint, "aria-pressed": shown(),
                    onclick: move |_| shown.set(!shown()),
                    svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor",
                        stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                        if shown() {
                            // eye with a slash: the password is visible, click to hide it
                            path { d: "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" }
                            line { x1: "1", y1: "1", x2: "23", y2: "23" }
                        } else {
                            path { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }
                            circle { cx: "12", cy: "12", r: "3" }
                        }
                    }
                }
            }
        }
    }
}

/// A `<select>` showing `value`: the matching option carries `selected`. `autocomplete="off"`
/// stops Firefox from restoring an older selection over the value the server just provided.
#[component]
pub fn Pick(
    id: &'static str,
    value: String,
    options: Vec<(String, String)>,
    onchange: EventHandler<String>,
) -> Element {
    rsx! {
        select { id, autocomplete: "off", onchange: move |e| onchange.call(e.value()),
            for (v, label) in options.iter() {
                option { key: "{v}", value: "{v}", selected: *v == value, "{label}" }
            }
        }
    }
}

/// Reduces a `ServerFnError` to the bare message, e.g.
/// `error running server function: invalid username or password (details: None)`.
pub fn clean_err(e: ServerFnError) -> String {
    let s = e.to_string();
    let s = s
        .strip_prefix("error running server function: ")
        .unwrap_or(&s);
    s.split(" (details:").next().unwrap_or(s).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_err_extracts_message() {
        let e = ServerFnError::ServerError {
            message: "invalid username or password".into(),
            code: 500,
            details: None,
        };
        assert_eq!(clean_err(e), "invalid username or password");
    }
}
