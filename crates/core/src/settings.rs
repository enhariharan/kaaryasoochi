// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use serde::{Deserialize, Serialize};

macro_rules! string_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident => $s:literal),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name { $($var),+ }
        impl Default for $name { fn default() -> Self { Self::$def } }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$var),+];
            pub fn as_str(&self) -> &'static str { match self { $(Self::$var => $s),+ } }
            /// Lenient parse; unknown values fall back to the default.
            pub fn parse(s: &str) -> Self {
                match s { $($s => Self::$var,)+ _ => Self::default() }
            }
        }
    };
}

string_enum!(Theme { Light => "light", Dark => "dark", System => "system" } default System);
string_enum!(Layout { Grid => "grid", List => "list" } default Grid);
string_enum!(TabOrientation { Horizontal => "horizontal", Vertical => "vertical" } default Horizontal);
string_enum!(
    /// `System` defers to the browser/OS locale on the client.
    DateFormat { System => "system", Iso => "%Y-%m-%d", Dmy => "%d/%m/%Y", Mdy => "%m/%d/%Y", Long => "%d %b %Y" }
    default System
);
string_enum!(TimeFormat { System => "system", H24 => "%H:%M", H12 => "%I:%M %p" } default System);

impl DateFormat {
    /// strftime pattern, or `None` for `System`.
    pub fn pattern(&self) -> Option<&'static str> {
        (*self != Self::System).then(|| self.as_str())
    }
}
impl TimeFormat {
    pub fn pattern(&self) -> Option<&'static str> {
        (*self != Self::System).then(|| self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_spec() {
        assert_eq!(Layout::default(), Layout::Grid);
        assert_eq!(TabOrientation::default(), TabOrientation::Horizontal);
        assert_eq!(Theme::default(), Theme::System);
    }
    #[test]
    fn roundtrip_and_fallback() {
        for l in Layout::ALL {
            assert_eq!(Layout::parse(l.as_str()), *l);
        }
        assert_eq!(Theme::parse("bogus"), Theme::System);
        assert_eq!(DateFormat::System.pattern(), None);
    }
}
