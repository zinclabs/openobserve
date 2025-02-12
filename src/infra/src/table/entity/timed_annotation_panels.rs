//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "timed_annotation_panels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub timed_annotation_id: String,
    pub panel_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::timed_annotations::Entity",
        from = "Column::TimedAnnotationId",
        to = "super::timed_annotations::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    TimedAnnotations,
}

impl Related<super::timed_annotations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TimedAnnotations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
