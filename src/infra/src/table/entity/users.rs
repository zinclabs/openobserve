//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    #[sea_orm(unique)]
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[sea_orm(column_type = "Text")]
    pub password: String,
    pub salt: String,
    pub is_root: bool,
    pub password_ext: Option<String>,
    pub user_type: i16,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::org_users::Entity")]
    OrgUsers,
}

impl Related<super::org_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
