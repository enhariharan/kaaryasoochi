// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! About page: name, version, author, license and source repository. The facts come from the
//! workspace's Cargo metadata (`[workspace.package]`), so a release only has to bump it there.

use dioxus::prelude::*;

use crate::state::t;

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Cargo joins several authors with `:`.
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const LICENSE: &str = env!("CARGO_PKG_LICENSE");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The repository URL as shown: without the scheme, e.g. `github.com/owner/repo`.
fn repository_label() -> &'static str {
    REPOSITORY
        .strip_prefix("https://")
        .unwrap_or(REPOSITORY)
        .trim_end_matches('/')
}

#[component]
pub fn About() -> Element {
    let authors = AUTHORS.replace(':', ", ");
    rsx! {
        section { class: "card about",
            div { class: "about-head",
                img { src: asset!("/assets/icon.svg"), width: "64", height: "64", alt: "", "aria-hidden": "true" }
                h1 { {t("app.name")} }
            }
            dl { class: "about-facts",
                dt { {t("about.version")} }
                dd { bdi { dir: "ltr", {VERSION} } }
                dt { {t("about.author")} }
                dd { bdi { dir: "auto", {authors} } }
                dt { {t("about.license")} }
                dd { bdi { dir: "ltr", {LICENSE} } }
                dt { {t("about.source")} }
                // Desktop: dioxus-desktop hands http(s) links to the default browser.
                dd { a { class: "mono", href: REPOSITORY, target: "_blank", rel: "noopener noreferrer",
                    {repository_label()} } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_metadata_is_set() {
        assert!(!VERSION.is_empty());
        assert!(!AUTHORS.is_empty(), "set authors in [workspace.package]");
        assert!(!LICENSE.is_empty());
        assert!(
            REPOSITORY.starts_with("https://github.com/"),
            "set repository in [workspace.package]: {REPOSITORY:?}"
        );
    }

    #[test]
    fn repository_label_drops_the_scheme() {
        assert_eq!(repository_label(), &REPOSITORY["https://".len()..]);
        assert!(!repository_label().contains("://"));
    }
}
