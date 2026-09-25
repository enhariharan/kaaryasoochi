// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use kaaryasoochi_core::dto::UserSettings;
use kaaryasoochi_core::limits::PAGE_SIZE_OPTIONS;
use kaaryasoochi_core::{DateFormat, Language, Layout, TabOrientation, Theme, TimeFormat};
use sea_orm::{prelude::*, ActiveValue::Set};

use crate::entity::user_settings as us;
use crate::{DbError, DbResult};

pub(crate) async fn create_defaults(db: &impl ConnectionTrait, user_id: i32) -> DbResult<()> {
    to_active(user_id, &UserSettings::default())
        .insert(db)
        .await?;
    Ok(())
}

fn to_active(user_id: i32, s: &UserSettings) -> us::ActiveModel {
    us::ActiveModel {
        user_id: Set(user_id),
        theme: Set(s.theme.as_str().into()),
        tab_orientation: Set(s.tab_orientation.as_str().into()),
        job_layout: Set(s.job_layout.as_str().into()),
        date_format: Set(s.date_format.as_str().into()),
        time_format: Set(s.time_format.as_str().into()),
        page_size: Set(s.page_size as i32),
        language: Set(s.language.code().into()),
    }
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
    us::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    to_active(user_id, &s).reset_all().update(db).await?;
    Ok(s)
}
