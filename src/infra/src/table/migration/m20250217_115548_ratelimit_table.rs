use sea_orm_migration::prelude::*;

const RULE_ID: &str = "rule_id_idx";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_ratelimit_table_statement())
            .await?;

        manager
            .create_index(create_ratelimit_rule_id_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RateLimitRules::Table).to_owned())
            .await
    }
}

fn create_ratelimit_table_statement() -> TableCreateStatement {
    Table::create()
        .table(RateLimitRules::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(RateLimitRules::RuleId)
                .string_len(256)
                .primary_key()
                .not_null(),
        )
        .col(
            ColumnDef::new(RateLimitRules::Org)
                .string_len(100)
                .not_null(),
        )
        .col(
            ColumnDef::new(RateLimitRules::RuleType)
                .string_len(50)
                .extra("DEFAULT 'exact'")
                .not_null(),
        )
        .col(
            ColumnDef::new(RateLimitRules::Resource)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(RateLimitRules::Threshold)
                .double()
                .not_null(),
        )
        .col(
            ColumnDef::new(RateLimitRules::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

fn create_ratelimit_rule_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(RULE_ID)
        .table(RateLimitRules::Table)
        .col(RateLimitRules::RuleId)
        .unique()
        .to_owned()
}

#[derive(DeriveIden)]
enum RateLimitRules {
    Table,
    Org,
    RuleId,
    RuleType,
    Resource,
    Threshold,
    CreatedAt,
}
