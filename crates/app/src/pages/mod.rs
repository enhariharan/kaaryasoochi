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

/// Inline error banner; renders nothing when there is no message.
#[component]
pub fn ErrorBanner(msg: ReadSignal<Option<String>>) -> Element {
    match msg() {
        Some(m) => rsx! { p { class: "error", role: "alert", "{m}" } },
        None => rsx! {},
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
