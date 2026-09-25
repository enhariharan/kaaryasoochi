// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Time-zone helpers. A user's setting is either [`SYSTEM`] (follow the device) or an IANA name.

pub use chrono_tz::Tz;

use crate::ValidationError;

/// Setting value meaning "use the device's time zone".
pub const SYSTEM: &str = "system";

pub fn parse(name: &str) -> Option<Tz> {
    name.parse().ok()
}

pub fn validate_setting(s: &str) -> Result<(), ValidationError> {
    if s == SYSTEM || parse(s).is_some() {
        Ok(())
    } else {
        Err(ValidationError::Timezone)
    }
}

/// All IANA names, for pickers.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    chrono_tz::TZ_VARIANTS.iter().map(|t| t.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates() {
        assert!(validate_setting("system").is_ok());
        assert!(validate_setting("Asia/Kolkata").is_ok());
        assert!(validate_setting("Mars/Base").is_err());
        assert!(all_names().any(|n| n == "Asia/Kolkata"));
    }
}
