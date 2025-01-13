use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ANNOTATION_PANELS_ANNOTATION_ID_IDX: &str = "annotation_panels_annotation_id_idx";
const ANNOTATION_PANELS_PANEL_ID_IDX: &str = "annotation_panels_panel_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table_stmt()).await?;
        manager
            .create_index(create_index_annotation_id_stmt())
            .await?;
        manager.create_index(create_index_panel_id_stmt()).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(ANNOTATION_PANELS_ANNOTATION_ID_IDX)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(ANNOTATION_PANELS_PANEL_ID_IDX)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(AnnotationPanels::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the annotation_panels table
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(AnnotationPanels::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(AnnotationPanels::Id)
                .string_len(64)
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(AnnotationPanels::TimedAnnotationId)
                .string_len(64)
                .not_null(),
        )
        .col(
            ColumnDef::new(AnnotationPanels::PanelId)
                .string_len(256)
                .not_null(),
        )
        .foreign_key(
            ForeignKey::create()
                .name("fk_annotation_panels_annotation_id")
                .from(AnnotationPanels::Table, AnnotationPanels::TimedAnnotationId)
                .to(TimedAnnotations::Table, TimedAnnotations::Id)
                .on_delete(ForeignKeyAction::Cascade),
        )
        .to_owned()
}

/// Statement to create an index on annotation_id
fn create_index_annotation_id_stmt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(ANNOTATION_PANELS_ANNOTATION_ID_IDX)
        .table(AnnotationPanels::Table)
        .col(AnnotationPanels::TimedAnnotationId)
        .to_owned()
}

/// Statement to create an index on panel_id
fn create_index_panel_id_stmt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(ANNOTATION_PANELS_PANEL_ID_IDX)
        .table(AnnotationPanels::Table)
        .col(AnnotationPanels::PanelId)
        .to_owned()
}

#[derive(DeriveIden)]
enum AnnotationPanels {
    Table,
    Id,
    TimedAnnotationId,
    PanelId,
}

#[derive(DeriveIden)]
enum TimedAnnotations {
    Table,
    Id,
}
