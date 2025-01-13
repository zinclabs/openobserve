use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ANNOTATIONS_DASHBOARD_ID_IDX: &str = "timed_annotations_dashboard_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table_stmt()).await?;
        manager
            .create_index(create_index_dashboard_id_stmt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(ANNOTATIONS_DASHBOARD_ID_IDX).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TimedAnnotations::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create table.
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(TimedAnnotations::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(TimedAnnotations::Id)
                .string_len(64)
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::DashboardId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::OrgId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::StartTime)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::EndTime)
                .big_integer()
                .null(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::Title)
                .string_len(256)
                .not_null(),
        )
        .col(ColumnDef::new(TimedAnnotations::Text).text().null())
        .col(
            ColumnDef::new(TimedAnnotations::Tags)
                .string_len(512)
                .null(),
        )
        .col(
            ColumnDef::new(TimedAnnotations::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

/// Statement to create index on dashboard_id.
fn create_index_dashboard_id_stmt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(ANNOTATIONS_DASHBOARD_ID_IDX)
        .table(TimedAnnotations::Table)
        .col(TimedAnnotations::DashboardId)
        .to_owned()
}

#[derive(DeriveIden)]
enum TimedAnnotations {
    Table,
    Id,
    DashboardId,
    OrgId,
    StartTime,
    EndTime,
    Title,
    Text,
    Tags,
    CreatedAt,
}
