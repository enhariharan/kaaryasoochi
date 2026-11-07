// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! "Forgot password": username -> secret question and answer -> new password (typed twice).
//! A correct answer only yields a short-lived token; the token, not the answer, authorises the
//! reset. Repeated wrong answers lock recovery for a while (enforced on the server).

use dioxus::prelude::*;
use kaaryasoochi_core::limits::{PASSWORD_MAX, PASSWORD_MIN, SECRET_ANSWER_MAX};

use super::{clean_err, ErrorBanner, IBtn, PasswordField};
use crate::icons::*;
use crate::state::t;
use crate::{api, Route};

#[derive(Clone, PartialEq)]
enum Step {
    Username,
    Answer { question: String },
    NewPassword { token: String },
    Done,
}

#[component]
pub fn Forgot() -> Element {
    let mut step = use_signal(|| Step::Username);
    let mut username = use_signal(String::new);
    let mut answer = use_signal(String::new);
    let (mut new, mut confirm) = (use_signal(String::new), use_signal(String::new));
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let start_over = move |_| {
        step.set(Step::Username);
        answer.set(String::new());
        new.set(String::new());
        confirm.set(String::new());
        error.set(None);
    };

    let ask = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::recovery_question(username()).await {
                Ok(question) => step.set(Step::Answer { question }),
                Err(e) => error.set(Some(clean_err(e))),
            }
            busy.set(false);
        });
    };

    let check = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::recovery_verify(username(), answer()).await {
                Ok(token) => {
                    answer.set(String::new()); // no longer needed, and should not linger
                    step.set(Step::NewPassword { token });
                }
                Err(e) => error.set(Some(clean_err(e))),
            }
            busy.set(false);
        });
    };

    let reset = move |e: FormEvent| {
        e.prevent_default();
        let Step::NewPassword { token } = step() else {
            return;
        };
        if new() != confirm() {
            error.set(Some(t("auth.password_mismatch").to_string()));
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::recovery_reset(token, new(), confirm()).await {
                Ok(()) => {
                    new.set(String::new());
                    confirm.set(String::new());
                    step.set(Step::Done);
                }
                Err(e) => error.set(Some(clean_err(e))),
            }
            busy.set(false);
        });
    };

    rsx! {
        section { class: "card auth",
            div { class: "auth-logo",
                img { src: asset!("/assets/icon.svg"), width: "72", height: "72", alt: "Kaaryasoochi" }
            }
            h1 { {t("auth.forgot.title")} }
            match step() {
                Step::Username => rsx! {
                    form { onsubmit: ask,
                        p { class: "muted", {t("auth.forgot.intro")} }
                        label { {t("auth.username")}
                            input { r#type: "text", autocomplete: "username", required: true, value: "{username}",
                                oninput: move |e| username.set(e.value()) } }
                        ErrorBanner { msg: error }
                        IBtn { icon: icon_arrow_right(), label: t("auth.forgot.continue"),
                            r#type: "submit", class: "primary", disabled: busy() }
                        p { class: "muted small", {t("auth.forgot.no_secret")} }
                    }
                },
                Step::Answer { question } => rsx! {
                    form { onsubmit: check,
                        div { class: "question",
                            small { class: "muted", {t("auth.secret_question")} }
                            p { dir: "auto", "{question}" }
                        }
                        PasswordField { label: "auth.secret_answer", value: answer, autocomplete: "off", maxlength: SECRET_ANSWER_MAX as i64 }
                        ErrorBanner { msg: error }
                        IBtn { icon: icon_check(), label: t("auth.forgot.check"),
                            r#type: "submit", class: "primary", disabled: busy() }
                        IBtn { icon: icon_refresh(), label: t("auth.forgot.start_over"),
                            r#type: "button", class: "link", onclick: start_over }
                    }
                },
                Step::NewPassword { .. } => rsx! {
                    form { onsubmit: reset,
                        PasswordField { label: "auth.new_password", value: new, autocomplete: "new-password",
                            minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64 }
                        PasswordField { label: "auth.confirm_password", value: confirm, autocomplete: "new-password",
                            minlength: PASSWORD_MIN as i64, maxlength: PASSWORD_MAX as i64 }
                        ErrorBanner { msg: error }
                        IBtn { icon: icon_lock(), label: t("auth.forgot.reset"),
                            r#type: "submit", class: "primary", disabled: busy() }
                        IBtn { icon: icon_refresh(), label: t("auth.forgot.start_over"),
                            r#type: "button", class: "link", onclick: start_over }
                    }
                },
                Step::Done => rsx! {
                    p { role: "status", {t("auth.forgot.done")} }
                },
            }
            p { class: "muted", Link { to: Route::Login {}, {icon_arrow_left()} " " {t("auth.forgot.back")} } }
        }
    }
}
