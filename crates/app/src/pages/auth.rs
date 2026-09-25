// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use dioxus::prelude::*;

use super::{clean_err, ErrorBanner};
use crate::state::{app_state, persist_token, t};
use crate::{api, Route};

#[component]
pub fn Login() -> Element {
    let mut s = app_state();
    let nav = use_navigator();
    let (mut username, mut password) = (use_signal(String::new), use_signal(String::new));
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    use_effect(move || {
        if s.token.read().is_some() {
            nav.replace(Route::Home {});
        }
    });

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        spawn(async move {
            let result = async {
                let tok = api::login(username(), password()).await?;
                let cfg = api::get_settings(tok.clone()).await?;
                Ok::<_, ServerFnError>((tok, cfg))
            }
            .await;
            match result {
                Ok((tok, cfg)) => {
                    persist_token(Some(&tok));
                    s.settings.set(cfg);
                    s.token.set(Some(tok));
                }
                Err(e) => error.set(Some(clean_err(e))),
            }
            busy.set(false);
        });
    };

    rsx! {
        section { class: "card auth",
            h1 { {t("auth.login")} }
            form { onsubmit: submit,
                label { {t("auth.username")}
                    input { r#type: "text", autocomplete: "username", required: true, value: "{username}", oninput: move |e| username.set(e.value()) } }
                label { {t("auth.password")}
                    input { r#type: "password", autocomplete: "current-password", required: true, value: "{password}", oninput: move |e| password.set(e.value()) } }
                ErrorBanner { msg: error }
                button { class: "primary", disabled: busy(), r#type: "submit", {t("auth.login")} }
            }
            // Passkey sign-in is planned; the button stays hidden until the WebAuthn flow exists.
            p { class: "muted", Link { to: Route::Register {}, {t("auth.register")} } }
        }
    }
}

#[component]
pub fn Register() -> Element {
    let nav = use_navigator();
    let (mut full_name, mut username, mut password) = (
        use_signal(String::new),
        use_signal(String::new),
        use_signal(String::new),
    );
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        spawn(async move {
            match api::register(username(), password(), full_name()).await {
                Ok(()) => {
                    nav.replace(Route::Login {});
                }
                Err(e) => error.set(Some(clean_err(e))),
            }
            busy.set(false);
        });
    };

    rsx! {
        section { class: "card auth",
            h1 { {t("auth.register")} }
            form { onsubmit: submit,
                label { {t("settings.full_name")}
                    input { r#type: "text", autocomplete: "name", maxlength: 100, value: "{full_name}", oninput: move |e| full_name.set(e.value()) } }
                label { {t("auth.username")}
                    input { r#type: "text", autocomplete: "username", required: true, minlength: 3, maxlength: 32, value: "{username}", oninput: move |e| username.set(e.value()) } }
                label { {t("auth.password")}
                    input { r#type: "password", autocomplete: "new-password", required: true, minlength: 10, maxlength: 128, value: "{password}", oninput: move |e| password.set(e.value()) } }
                ErrorBanner { msg: error }
                button { class: "primary", disabled: busy(), r#type: "submit", {t("auth.register")} }
            }
            p { class: "muted", Link { to: Route::Login {}, {t("auth.login")} } }
        }
    }
}
