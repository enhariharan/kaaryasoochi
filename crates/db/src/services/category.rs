// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use kaaryasoochi_core::dto::CategoryView;
use kaaryasoochi_core::limits::CATEGORY_MAX;
use kaaryasoochi_core::{ValidationError, DEFAULT_CATEGORY};
use sea_orm::{prelude::*, sea_query::Expr, ActiveValue::Set, QueryOrder};

use crate::entity::{category, job};
use crate::{DbError, DbResult};

fn view(m: category::Model) -> CategoryView {
    CategoryView {
        id: m.id,
        name: m.name,
    }
}

pub(crate) async fn ensure_default(
    db: &impl ConnectionTrait,
    user_id: i32,
) -> DbResult<category::Model> {
    get_or_create(db, user_id, DEFAULT_CATEGORY).await
}

/// Case-insensitive lookup; creates the category when absent. Blank means `Default`.
pub(crate) async fn get_or_create(
    db: &impl ConnectionTrait,
    user_id: i32,
    name: &str,
) -> DbResult<category::Model> {
    let name = match name.trim() {
        "" => DEFAULT_CATEGORY,
        n => n,
    };
    if name.chars().count() > CATEGORY_MAX {
        return Err(ValidationError::CategoryTooLong.into());
    }
    let existing = category::Entity::find()
        .filter(category::Column::UserId.eq(user_id))
        .filter(Expr::cust_with_values("lower(name) = lower(?)", [name]))
        .one(db)
        .await?;
    if let Some(c) = existing {
        return Ok(c);
    }
    Ok(category::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.into()),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

/// Alphabetical, with `Default` always last (so it is the far end of the dashboard tab strip:
/// right-most in left-to-right languages, left-most in right-to-left ones).
pub async fn list(db: &impl ConnectionTrait, user_id: i32) -> DbResult<Vec<CategoryView>> {
    let mut v: Vec<_> = category::Entity::find()
        .filter(category::Column::UserId.eq(user_id))
        .order_by_asc(category::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(view)
        .collect();
    v.sort_by_key(|c| c.name == DEFAULT_CATEGORY); // false < true, and the sort is stable
    Ok(v)
}

/// Jobs in the deleted category move to `Default`.
pub async fn delete(db: &impl ConnectionTrait, user_id: i32, id: i32) -> DbResult<()> {
    let c = category::Entity::find_by_id(id)
        .filter(category::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    if c.name == DEFAULT_CATEGORY {
        return Err(DbError::ProtectedCategory);
    }
    let default = ensure_default(db, user_id).await?;
    job::Entity::update_many()
        .col_expr(job::Column::CategoryId, Expr::value(default.id))
        .filter(job::Column::CategoryId.eq(id))
        .exec(db)
        .await?;
    category::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
