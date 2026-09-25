// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i32,
    pub theme: String,
    pub tab_orientation: String,
    pub job_layout: String,
    pub date_format: String,
    pub time_format: String,
    pub page_size: i32,
    pub language: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
