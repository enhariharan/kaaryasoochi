//! Wire types shared between server functions and the UI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{DateFormat, Language, Layout, Repeat, TabOrientation, Theme, TimeFormat};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserView {
    pub id: i32,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryView {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobView {
    pub id: i32,
    pub title: String,
    pub category: String,
    pub summary: String,
    pub description: String,
    pub first_run: DateTime<Utc>,
    pub repeat: Option<Repeat>,
    pub next_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "failed" => Self::Failed,
            _ => Self::Running,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunView {
    pub id: i32,
    pub scheduled_for: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: RunStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

impl<T> Page<T> {
    pub fn total_pages(&self) -> u32 {
        (self.total.div_ceil(self.page_size.max(1) as u64)).max(1) as u32
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
    pub theme: Theme,
    pub tab_orientation: TabOrientation,
    pub job_layout: Layout,
    pub date_format: DateFormat,
    pub time_format: TimeFormat,
    pub page_size: u32,
    pub language: Language,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            tab_orientation: TabOrientation::default(),
            job_layout: Layout::default(),
            date_format: DateFormat::default(),
            time_format: TimeFormat::default(),
            page_size: crate::limits::PAGE_SIZE_DEFAULT,
            language: Language::default(),
        }
    }
}
