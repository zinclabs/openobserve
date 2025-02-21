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

use config::meta::meta_store::MetaStore;
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
mod m20250109_092400_recreate_tables_with_ksuids;
mod m20250113_144600_create_unique_folder_name_idx;
mod m20250121_120000_create_cipher_table;
mod m20250122_000001_create_table_action_scripts;
mod m20250124_000001_create_timed_annotations_table;
mod m20250124_000002_create_timed_annotation_panels_table;
mod m20250125_102300_create_destinations_table;
mod m20250125_115400_create_templates_table;
mod m20250125_132500_populate_templates_table;
mod m20250125_133700_populate_destinations_table;
mod m20250125_153005_delete_metas_destinations;
mod m20250125_172300_delete_metas_templates;
mod m20250213_000001_add_dashboard_updated_at;
mod m20250217_115548_ratelimit_table;

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
            Box::new(m20250113_144600_create_unique_folder_name_idx::Migration),
            Box::new(m20250121_120000_create_cipher_table::Migration),
            Box::new(m20250122_000001_create_table_action_scripts::Migration),
            Box::new(m20250124_000001_create_timed_annotations_table::Migration),
            Box::new(m20250124_000002_create_timed_annotation_panels_table::Migration),
            Box::new(m20250125_115400_create_templates_table::Migration),
            Box::new(m20250125_132500_populate_templates_table::Migration),
            Box::new(m20250125_172300_delete_metas_templates::Migration),
            Box::new(m20250125_102300_create_destinations_table::Migration),
            Box::new(m20250125_133700_populate_destinations_table::Migration),
            Box::new(m20250125_153005_delete_metas_destinations::Migration),
            Box::new(m20250213_000001_add_dashboard_updated_at::Migration),
            Box::new(m20250217_115548_ratelimit_table::Migration),
        ]
    }
}

pub fn get_text_type() -> String {
    let db_type = config::get_config().common.meta_store.as_str().into();
    match db_type {
        MetaStore::MySQL => config::get_config().limit.db_text_data_type.clone(),
        _ => "text".to_string(),
    }
}
