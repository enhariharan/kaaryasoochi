use sea_orm::entity::prelude::*;

/// WebAuthn credential. Schema is in place; the ceremony flow is not implemented yet.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "passkey")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    #[sea_orm(unique)]
    pub credential_id: String,
    /// Serialized public-key credential (JSON).
    pub credential: String,
    pub label: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
