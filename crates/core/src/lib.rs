//! Pure domain logic shared by the server and every UI target.
//! No I/O, no database, no UI: everything here is unit-testable.

pub mod dto;
pub mod i18n;
pub mod limits;
pub mod recurrence;
pub mod settings;
pub mod validation;

pub use i18n::{tr, Language};
pub use recurrence::{Repeat, Weekday};
pub use settings::{DateFormat, Layout, TabOrientation, Theme, TimeFormat};
pub use validation::{JobInput, ValidationError};

/// Name of the category that always exists and receives uncategorised jobs.
pub const DEFAULT_CATEGORY: &str = "Default";
