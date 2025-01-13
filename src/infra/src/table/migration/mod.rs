// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

pub use sea_orm_migration::prelude::*;

mod m20241114_000001_create_folders_table;
mod m20241115_150000_populate_folders_table;
mod m20241116_000001_delete_metas;
mod m20241116_000002_drop_folders_created_at_column;
mod m20241119_000001_create_dashboards_table;
mod m20241119_000002_populate_dashboards_table;
mod m20241119_000003_delete_metas;
mod m20241204_143100_create_table_search_queue;
mod m20241209_120000_create_alerts_table;
mod m20241215_190333_delete_metas;
mod m20241217_154900_alter_folders_table_idx;
mod m20241217_155000_populate_alerts_table;
mod m20241222_085111_search_jobs;
mod m20241222_085135_search_job_partitions;
mod m20241222_085148_search_job_results;
mod m20241224_120000_create_cipher_table;
mod m20250109_092400_recreate_tables_with_ksuids;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241114_000001_create_folders_table::Migration),
            Box::new(m20241115_150000_populate_folders_table::Migration),
            Box::new(m20241116_000001_delete_metas::Migration),
            Box::new(m20241116_000002_drop_folders_created_at_column::Migration),
            Box::new(m20241119_000001_create_dashboards_table::Migration),
            Box::new(m20241119_000002_populate_dashboards_table::Migration),
            Box::new(m20241119_000003_delete_metas::Migration),
            Box::new(m20241204_143100_create_table_search_queue::Migration),
            Box::new(m20241209_120000_create_alerts_table::Migration),
            Box::new(m20241215_190333_delete_metas::Migration),
            Box::new(m20241217_154900_alter_folders_table_idx::Migration),
            Box::new(m20241217_155000_populate_alerts_table::Migration),
            Box::new(m20241222_085111_search_jobs::Migration),
            Box::new(m20241222_085135_search_job_partitions::Migration),
            Box::new(m20241222_085148_search_job_results::Migration),
            Box::new(m20250109_092400_recreate_tables_with_ksuids::Migration),
            Box::new(m20241224_120000_create_cipher_table::Migration),
        ]
    }
}
