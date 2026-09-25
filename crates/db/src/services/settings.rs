// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::limits::PAGE_SIZE_OPTIONS;
use kaaryasoochi_core::timezone::{self, Tz};
use kaaryasoochi_core::{DateFormat, Language, Layout, TabOrientation, Theme, TimeFormat};
use sea_orm::{prelude::*, ActiveValue::Set};

use crate::entity::user_settings as us;
use crate::{DbError, DbResult};

pub(crate) async fn create_defaults(db: &impl ConnectionTrait, user_id: i32) -> DbResult<()> {
    us::ActiveModel {
        user_id: Set(user_id),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn get(db: &impl ConnectionTrait, user_id: i32) -> DbResult<UserSettings> {
    let m = us::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(UserSettings {
        theme: Theme::parse(&m.theme),
        tab_orientation: TabOrientation::parse(&m.tab_orientation),
        job_layout: Layout::parse(&m.job_layout),
        date_format: DateFormat::parse(&m.date_format),
        time_format: TimeFormat::parse(&m.time_format),
        page_size: m.page_size.max(1) as u32,
        language: Language::parse(&m.language),
        timezone: m.timezone,
    })
}

pub async fn update(
    db: &impl ConnectionTrait,
    user_id: i32,
    s: UserSettings,
) -> DbResult<UserSettings> {
    if !PAGE_SIZE_OPTIONS.contains(&s.page_size) {
        return Err(DbError::Internal(format!(
            "page size must be one of {PAGE_SIZE_OPTIONS:?}"
        )));
    }
    timezone::validate_setting(&s.timezone)?;
    let existing = us::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    let mut am: us::ActiveModel = existing.into();
    am.theme = Set(s.theme.as_str().into());
    am.tab_orientation = Set(s.tab_orientation.as_str().into());
    am.job_layout = Set(s.job_layout.as_str().into());
    am.date_format = Set(s.date_format.as_str().into());
    am.time_format = Set(s.time_format.as_str().into());
    am.page_size = Set(s.page_size as i32);
    am.language = Set(s.language.code().into());
    am.timezone = Set(s.timezone.clone());
    am.update(db).await?;
    Ok(s)
}

/// Remembers the zone the user's device reports (used while the setting is `system`).
pub async fn set_system_timezone(
    db: &impl ConnectionTrait,
    user_id: i32,
    name: &str,
) -> DbResult<()> {
    timezone::parse(name).ok_or(kaaryasoochi_core::ValidationError::Timezone)?;
    let existing = us::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    if existing.system_timezone != name {
        let mut am: us::ActiveModel = existing.into();
        am.system_timezone = Set(name.to_owned());
        am.update(db).await?;
    }
    Ok(())
}

/// The zone new schedules are evaluated in: the explicit setting, else the device's reported
/// zone, else this server's own zone, else UTC.
pub async fn effective_timezone(db: &impl ConnectionTrait, user_id: i32) -> DbResult<Tz> {
    let m = us::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    let pick = if m.timezone == timezone::SYSTEM {
        timezone::parse(&m.system_timezone)
            .or_else(|| {
                iana_time_zone::get_timezone()
                    .ok()
                    .and_then(|n| timezone::parse(&n))
            })
            .unwrap_or(Tz::UTC)
    } else {
        timezone::parse(&m.timezone).unwrap_or(Tz::UTC)
    };
    Ok(pick)
}
