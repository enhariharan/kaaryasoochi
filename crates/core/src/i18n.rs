// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Static-text translations.
//!
//! Each language is one plain-text file in `crates/core/locales/<code>.txt` with a
//! `key = text` line per string, compiled into the binary with `include_str!`. `en.txt` is the
//! reference: tests require every other locale to define exactly the same keys, with no empty
//! text, and every `t("key")` used by the UI to exist. Lookup falls back to English, then to "".
//!
//! Each file also declares its text direction with an `@dir = ltr|rtl` line; the UI sets `dir`
//! (and mirrors its layout) from it, so a right-to-left language needs no layout code.
//!
//! To add a language: add a `locales/<code>.txt` (with `@dir`), a `Language` variant, and its
//! entries in `Language::{ALL, code, native_name}` and `LOCALE_SOURCES`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    En,
    Ta,
    Ml,
    Te,
    Hi,
    Sa,
    Ur,
}

impl Language {
    pub const ALL: [Language; 7] = [
        Self::En,
        Self::Ta,
        Self::Ml,
        Self::Te,
        Self::Hi,
        Self::Sa,
        Self::Ur,
    ];

    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ta => "ta",
            Self::Ml => "ml",
            Self::Te => "te",
            Self::Hi => "hi",
            Self::Sa => "sa",
            Self::Ur => "ur",
        }
    }

    /// Name in its own script, for the language picker.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ta => "தமிழ்",
            Self::Ml => "മലയാളം",
            Self::Te => "తెలుగు",
            Self::Hi => "हिन्दी",
            Self::Sa => "संस्कृतम्",
            Self::Ur => "اردو",
        }
    }

    pub fn parse(code: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|l| l.code() == code)
            .unwrap_or_default()
    }

    fn idx(&self) -> usize {
        Self::ALL.iter().position(|l| l == self).unwrap()
    }

    /// Text direction declared by the locale file (`ltr` unless it says `@dir = rtl`).
    pub fn dir(&self) -> &'static str {
        if locale_is_rtl(LOCALE_SOURCES[self.idx()]) {
            "rtl"
        } else {
            "ltr"
        }
    }

    pub fn is_rtl(&self) -> bool {
        self.dir() == "rtl"
    }
}

/// True when the locale source declares `@dir = rtl`.
fn locale_is_rtl(src: &str) -> bool {
    src.lines()
        .filter_map(|l| l.trim().strip_prefix("@dir"))
        .filter_map(|rest| rest.trim().strip_prefix('='))
        .any(|v| v.trim().eq_ignore_ascii_case("rtl"))
}

/// Locale files, in `Language::ALL` order.
const LOCALE_SOURCES: [&str; 7] = [
    include_str!("../locales/en.txt"),
    include_str!("../locales/ta.txt"),
    include_str!("../locales/ml.txt"),
    include_str!("../locales/te.txt"),
    include_str!("../locales/hi.txt"),
    include_str!("../locales/sa.txt"),
    include_str!("../locales/ur.txt"),
];

type Catalog = HashMap<&'static str, &'static str>;

/// Parses one locale file. Malformed lines are reported by the tests via [`parse_locale`].
fn parse_locale(src: &'static str) -> Result<Vec<(&'static str, &'static str)>, String> {
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let line = line.trim_end();
        let trimmed = line.trim_start();
        // Blank lines, comments and `@meta = value` directives carry no translation.
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            continue;
        }
        let (key, text) = line
            .split_once(" = ")
            .ok_or_else(|| format!("line {}: expected `key = text`: {line}", n + 1))?;
        out.push((key.trim(), text.trim()));
    }
    Ok(out)
}

fn catalogs() -> &'static [Catalog; 7] {
    static CATALOGS: OnceLock<[Catalog; 7]> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        LOCALE_SOURCES.map(|src| {
            // Malformed lines are skipped here (a test guarantees there are none).
            parse_locale(src).unwrap_or_default().into_iter().collect()
        })
    })
}

/// Look up `key`; falls back to English, then to the empty string.
pub fn tr(lang: Language, key: &str) -> &'static str {
    let c = catalogs();
    c[lang.idx()]
        .get(key)
        .or_else(|| c[0].get(key))
        .copied()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn keys_of(src: &'static str) -> Vec<&'static str> {
        parse_locale(src)
            .unwrap()
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    #[test]
    fn locale_files_are_well_formed() {
        for (lang, src) in Language::ALL.iter().zip(LOCALE_SOURCES) {
            let entries = parse_locale(src).unwrap_or_else(|e| panic!("{}: {e}", lang.code()));
            assert!(!entries.is_empty(), "{} is empty", lang.code());
            let mut seen = BTreeSet::new();
            for (k, v) in entries {
                assert!(!v.is_empty(), "{}: empty text for {k}", lang.code());
                assert!(seen.insert(k), "{}: duplicate key {k}", lang.code());
            }
        }
    }

    #[test]
    fn every_locale_has_exactly_the_english_keys() {
        let en: BTreeSet<_> = keys_of(LOCALE_SOURCES[0]).into_iter().collect();
        for (lang, src) in Language::ALL.iter().zip(LOCALE_SOURCES).skip(1) {
            let have: BTreeSet<_> = keys_of(src).into_iter().collect();
            let missing: Vec<_> = en.difference(&have).collect();
            let extra: Vec<_> = have.difference(&en).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{}: missing {missing:?}, unknown {extra:?}",
                lang.code()
            );
        }
    }

    /// Non-English text must actually be translated, not a copy of the English (allowed for
    /// brand names, numbers and symbols only).
    #[test]
    fn translations_are_not_english_copies() {
        let en: HashMap<_, _> = parse_locale(LOCALE_SOURCES[0])
            .unwrap()
            .into_iter()
            .collect();
        for (lang, src) in Language::ALL.iter().zip(LOCALE_SOURCES).skip(1) {
            for (k, v) in parse_locale(src).unwrap() {
                let ascii_letters = v.chars().filter(|c| c.is_ascii_alphabetic()).count();
                let letters = v.chars().filter(|c| c.is_alphabetic()).count();
                // Mostly-Latin text equal to English means "forgot to translate".
                assert!(
                    !(v == en[k]
                        && letters > 0
                        && ascii_letters == letters
                        && !["app.name"].contains(&k)),
                    "{}: {k} is untranslated: {v}",
                    lang.code()
                );
            }
        }
    }

    /// Every literal `t("key")` / `tr(lang, "key")` used by the UI must exist; a missing key
    /// silently renders as an empty string.
    #[test]
    fn every_key_used_by_the_ui_exists() {
        fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    rust_files(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let en: BTreeSet<_> = keys_of(LOCALE_SOURCES[0]).into_iter().collect();
        let mut files = Vec::new();
        rust_files(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/src"),
            &mut files,
        );
        assert!(!files.is_empty(), "no UI sources found");
        let mut missing = Vec::new();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            for marker in ["t(\"", "tr(lang, \""] {
                for (i, _) in src.match_indices(marker) {
                    // `t(` must be a call, not the tail of another identifier such as `format(`.
                    if marker == "t(\""
                        && src[..i]
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                    {
                        continue;
                    }
                    let rest = &src[i + marker.len()..];
                    let key = &rest[..rest.find('"').unwrap()];
                    if !en.contains(key) {
                        missing.push(format!(
                            "{key} ({})",
                            f.file_name().unwrap().to_string_lossy()
                        ));
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "translation keys missing from en.txt: {missing:?}"
        );
    }

    #[test]
    fn every_locale_declares_its_direction() {
        for (lang, src) in Language::ALL.iter().zip(LOCALE_SOURCES) {
            assert!(
                src.lines().any(|l| l.trim().starts_with("@dir")),
                "{} has no @dir line",
                lang.code()
            );
            // Urdu is the only right-to-left language shipped.
            let rtl = *lang == Language::Ur;
            assert_eq!(
                (lang.is_rtl(), lang.dir()),
                (rtl, if rtl { "rtl" } else { "ltr" }),
                "{}",
                lang.code()
            );
        }
    }

    #[test]
    fn direction_directive_is_parsed() {
        assert!(locale_is_rtl("# c\n@dir = rtl\nkey = v\n"));
        assert!(locale_is_rtl("  @dir=RTL  "));
        assert!(!locale_is_rtl("@dir = ltr\nkey = v"));
        assert!(
            !locale_is_rtl("key = @dir = rtl"),
            "only a directive line counts"
        );
        assert!(!locale_is_rtl("key = v"), "default is ltr");
        // Directives never leak into the translation keys.
        let parsed = parse_locale("@dir = rtl\nkey = v\n").unwrap();
        assert_eq!(parsed, [("key", "v")]);
    }

    /// The stylesheet must mirror under `dir="rtl"`, so it may only use logical properties
    /// (`margin-inline-start`, `inset-inline-end`, `text-align: start`, ...), never physical
    /// left/right ones. (Comments are ignored.)
    #[test]
    fn stylesheet_uses_logical_properties_only() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/assets/main.css");
        let mut css = std::fs::read_to_string(path).unwrap();
        while let Some(a) = css.find("/*") {
            let b = css[a..].find("*/").map(|i| a + i + 2).unwrap_or(css.len());
            css.replace_range(a..b, "");
        }
        let css: String = css.split_whitespace().collect::<Vec<_>>().join(" ");
        let banned = [
            "margin-left",
            "margin-right",
            "padding-left",
            "padding-right",
            "border-left",
            "border-right",
            "text-align:left",
            "text-align: left",
            "text-align:right",
            "text-align: right",
            " left:",
            " left :",
            " right:",
            " right :",
            "{left:",
            "{right:",
            ";left:",
            ";right:",
            "float:left",
            "float: left",
            "float:right",
            "float: right",
        ];
        // Native <select> widgets ignored the theme in WebKitGTK (unreadable text in dark themes),
        // so the stylesheet draws them itself and declares a colour scheme per theme.
        assert!(
            css.contains("select { appearance:none"),
            "select must be drawn by the stylesheet"
        );
        assert!(
            css.contains("color-scheme: dark"),
            "dark themes must declare color-scheme: dark"
        );
        let found: Vec<_> = banned.iter().filter(|b| css.contains(**b)).collect();
        assert!(
            found.is_empty(),
            "physical direction properties in main.css: {found:?}"
        );
    }

    #[test]
    fn lookup_fallback_and_language_codes() {
        assert_eq!(tr(Language::En, "auth.login"), "Log in");
        assert_eq!(tr(Language::Hi, "auth.login"), "लॉग इन");
        assert_eq!(tr(Language::Sa, "auth.login"), "प्रविशतु");
        assert_eq!(tr(Language::Ur, "auth.login"), "لاگ اِن");
        assert_eq!(Language::parse("ur"), Language::Ur);
        assert_eq!(Language::Ur.native_name(), "اردو");
        assert_eq!(tr(Language::Ta, "missing.key"), "");
        assert_eq!(Language::parse("sa"), Language::Sa);
        assert_eq!(Language::parse("xx"), Language::En);
        for l in Language::ALL {
            assert_eq!(Language::parse(l.code()), l);
        }
        assert_eq!(Language::Sa.native_name(), "संस्कृतम्");
    }
}
