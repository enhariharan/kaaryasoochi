// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden, Clone, Copy)]
enum User {
    Table,
    Id,
    Username,
    FullName,
    PasswordHash,
    CreatedAt,
}
#[derive(DeriveIden, Clone, Copy)]
enum Session {
    Table,
    TokenHash,
    UserId,
    ExpiresAt,
}
#[derive(DeriveIden, Clone, Copy)]
enum Passkey {
    Table,
    Id,
    UserId,
    CredentialId,
    Credential,
    Label,
    CreatedAt,
}
#[derive(DeriveIden, Clone, Copy)]
enum UserSettings {
    Table,
    UserId,
    Theme,
    TabOrientation,
    JobLayout,
    DateFormat,
    TimeFormat,
    PageSize,
    Language,
}
#[derive(DeriveIden, Clone, Copy)]
enum Category {
    Table,
    Id,
    UserId,
    Name,
}
#[derive(DeriveIden, Clone, Copy)]
enum Job {
    Table,
    Id,
    UserId,
    CategoryId,
    Title,
    Summary,
    Description,
    FirstRun,
    RepeatKind,
    RepeatValue,
    NextRun,
    CreatedAt,
}
#[derive(DeriveIden, Clone, Copy)]
enum JobRun {
    Table,
    Id,
    JobId,
    ScheduledFor,
    StartedAt,
    FinishedAt,
    Status,
    Message,
}

fn fk(
    name: &str,
    from_t: impl IntoIden + Clone + 'static,
    from_c: impl IntoIden + Clone + 'static,
    to_t: impl IntoIden + Clone + 'static,
    to_c: impl IntoIden + Clone + 'static,
) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .name(name)
        .from(from_t, from_c)
        .to(to_t, to_c)
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(User::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(User::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(User::Username)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(User::FullName)
                        .string()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(User::PasswordHash).string().not_null())
                .col(ColumnDef::new(User::CreatedAt).timestamp().not_null())
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(Session::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Session::TokenHash)
                        .string()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Session::UserId).integer().not_null())
                .col(ColumnDef::new(Session::ExpiresAt).timestamp().not_null())
                .foreign_key(&mut fk(
                    "fk_session_user",
                    Session::Table,
                    Session::UserId,
                    User::Table,
                    User::Id,
                ))
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(Passkey::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Passkey::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Passkey::UserId).integer().not_null())
                .col(
                    ColumnDef::new(Passkey::CredentialId)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Passkey::Credential).text().not_null())
                .col(
                    ColumnDef::new(Passkey::Label)
                        .string()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Passkey::CreatedAt).timestamp().not_null())
                .foreign_key(&mut fk(
                    "fk_passkey_user",
                    Passkey::Table,
                    Passkey::UserId,
                    User::Table,
                    User::Id,
                ))
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(UserSettings::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(UserSettings::UserId)
                        .integer()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(UserSettings::Theme)
                        .string()
                        .not_null()
                        .default("system"),
                )
                .col(
                    ColumnDef::new(UserSettings::TabOrientation)
                        .string()
                        .not_null()
                        .default("horizontal"),
                )
                .col(
                    ColumnDef::new(UserSettings::JobLayout)
                        .string()
                        .not_null()
                        .default("grid"),
                )
                .col(
                    ColumnDef::new(UserSettings::DateFormat)
                        .string()
                        .not_null()
                        .default("system"),
                )
                .col(
                    ColumnDef::new(UserSettings::TimeFormat)
                        .string()
                        .not_null()
                        .default("system"),
                )
                .col(
                    ColumnDef::new(UserSettings::PageSize)
                        .integer()
                        .not_null()
                        .default(10),
                )
                .col(
                    ColumnDef::new(UserSettings::Language)
                        .string()
                        .not_null()
                        .default("en"),
                )
                .foreign_key(&mut fk(
                    "fk_settings_user",
                    UserSettings::Table,
                    UserSettings::UserId,
                    User::Table,
                    User::Id,
                ))
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(Category::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Category::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Category::UserId).integer().not_null())
                .col(ColumnDef::new(Category::Name).string().not_null())
                .foreign_key(&mut fk(
                    "fk_category_user",
                    Category::Table,
                    Category::UserId,
                    User::Table,
                    User::Id,
                ))
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_category_user_name")
                .table(Category::Table)
                .col(Category::UserId)
                .col(Category::Name)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(Job::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Job::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Job::UserId).integer().not_null())
                .col(ColumnDef::new(Job::CategoryId).integer().not_null())
                .col(ColumnDef::new(Job::Title).string().not_null())
                .col(ColumnDef::new(Job::Summary).string().not_null().default(""))
                .col(
                    ColumnDef::new(Job::Description)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Job::FirstRun).timestamp().not_null())
                .col(ColumnDef::new(Job::RepeatKind).string().null())
                .col(ColumnDef::new(Job::RepeatValue).integer().null())
                .col(ColumnDef::new(Job::NextRun).timestamp().null())
                .col(ColumnDef::new(Job::CreatedAt).timestamp().not_null())
                .foreign_key(&mut fk(
                    "fk_job_user",
                    Job::Table,
                    Job::UserId,
                    User::Table,
                    User::Id,
                ))
                .foreign_key(&mut fk(
                    "fk_job_category",
                    Job::Table,
                    Job::CategoryId,
                    Category::Table,
                    Category::Id,
                ))
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_job_next_run")
                .table(Job::Table)
                .col(Job::NextRun)
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_job_category")
                .table(Job::Table)
                .col(Job::CategoryId)
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(JobRun::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(JobRun::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(JobRun::JobId).integer().not_null())
                .col(ColumnDef::new(JobRun::ScheduledFor).timestamp().not_null())
                .col(ColumnDef::new(JobRun::StartedAt).timestamp().not_null())
                .col(ColumnDef::new(JobRun::FinishedAt).timestamp().null())
                .col(ColumnDef::new(JobRun::Status).string().not_null())
                .col(
                    ColumnDef::new(JobRun::Message)
                        .text()
                        .not_null()
                        .default(""),
                )
                .foreign_key(&mut fk(
                    "fk_run_job",
                    JobRun::Table,
                    JobRun::JobId,
                    Job::Table,
                    Job::Id,
                ))
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_run_job_started")
                .table(JobRun::Table)
                .col(JobRun::JobId)
                .col(JobRun::StartedAt)
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for t in [
            JobRun::Table.into_iden(),
            Job::Table.into_iden(),
            Category::Table.into_iden(),
            UserSettings::Table.into_iden(),
            Passkey::Table.into_iden(),
            Session::Table.into_iden(),
            User::Table.into_iden(),
        ] {
            m.drop_table(Table::drop().table(t).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
}
