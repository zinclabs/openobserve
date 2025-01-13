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

use chrono::Utc;
use config::{ider, meta::timed_annotations::*};
use sea_orm::{
    prelude::Expr, ColumnTrait, Condition, EntityTrait, QueryFilter, Set, TransactionTrait,
};

use super::{
    entity::{
        annotation_panels,
        timed_annotations::{self, *},
    },
    get_lock,
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

pub async fn get(
    dashboard_id: &str,
    panel_ids: Option<Vec<String>>,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<TimedAnnotation>, errors::Error> {
    use sea_orm::{entity::*, query::*, sea_query::JoinType};

    // make sure only one client is writing to the database (only for SQLite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // Step 1: Build the query for `timed_annotations` with a JOIN on `annotation_panels`
    let mut query = timed_annotations::Entity::find()
        .filter(timed_annotations::Column::DashboardId.eq(dashboard_id))
        .join(
            JoinType::InnerJoin,
            timed_annotations::Relation::AnnotationPanels.def(),
        );

    // Step 2: If `panel_ids` is provided, filter by `panel_id`
    if let Some(panel_ids) = panel_ids {
        query = query.filter(
            Expr::col((
                annotation_panels::Entity,
                annotation_panels::Column::PanelId,
            ))
            .is_in(panel_ids),
        );
    }

    // Step 3: Filter by time range (overlap condition)
    // Handle entries where end_time is null and ensure start_time is within the query range
    query = query
        .filter(
            Expr::col((
                timed_annotations::Entity,
                timed_annotations::Column::StartTime,
            ))
            .lte(end_time), // annotation.start_time <= end_time
        )
        .filter(
            Condition::any()
                .add(
                    Expr::col((
                        timed_annotations::Entity,
                        timed_annotations::Column::EndTime,
                    ))
                    .gte(start_time), // annotation.end_time >= start_time
                )
                .add(
                    Condition::all()
                        .add(
                            Expr::col((
                                timed_annotations::Entity,
                                timed_annotations::Column::EndTime,
                            ))
                            .is_null(), // end_time is null
                        )
                        .add(
                            Expr::col((
                                timed_annotations::Entity,
                                timed_annotations::Column::StartTime,
                            ))
                            .gte(start_time), // annotation.start_time >= start_time
                        ),
                ),
        );

    // Step 4: Select distinct annotations
    let annotations = query
        .distinct_on([timed_annotations::Column::Id])
        .all(client)
        .await?;

    if annotations.is_empty() {
        return Ok(vec![]); // No annotations found
    }

    // Step 5: Collect annotation IDs
    let annotation_ids: Vec<String> = annotations.iter().map(|a| a.id.clone()).collect();

    // Step 6: Fetch associated panels for the retrieved annotations
    let panels = annotation_panels::Entity::find()
        .filter(annotation_panels::Column::TimedAnnotationId.is_in(annotation_ids.clone()))
        .all(client)
        .await?;

    // Step 7: Group panel IDs by annotation ID
    let mut panels_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for panel in panels {
        panels_map
            .entry(panel.timed_annotation_id.clone())
            .or_insert_with(Vec::new)
            .push(panel.panel_id.clone());
    }

    // Step 8: Combine annotations with their associated panel IDs
    let results = annotations
        .into_iter()
        .map(|annotation| TimedAnnotation {
            panels: panels_map
                .get(&annotation.id)
                .cloned()
                .unwrap_or_else(Vec::new),
            annotation_id: Some(annotation.id),
            start_time: annotation.start_time,
            end_time: annotation.end_time,
            title: annotation.title,
            text: annotation.text,
            tags: annotation.tags,
        })
        .collect();

    Ok(results)
}

pub async fn add(
    dashboard_id: &str,
    org_id: &str,
    timed_annotation: TimedAnnotation,
) -> Result<String, errors::Error> {
    let record = ActiveModel {
        id: Set(ider::uuid()),
        dashboard_id: Set(dashboard_id.to_string()),
        start_time: Set(timed_annotation.start_time),
        end_time: Set(timed_annotation.end_time),
        title: Set(timed_annotation.title),
        text: Set(timed_annotation.text),
        tags: Set(timed_annotation.tags),
        created_at: Set(Utc::now().timestamp_micros()),
        org_id: Set(org_id.to_string()),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let insert_result = Entity::insert(record).exec(client).await?;
    let timed_annotation_id = insert_result.last_insert_id;

    // Insert into the annotation_panels table
    for panel_id in timed_annotation.panels {
        let record = annotation_panels::ActiveModel {
            id: (Set(panel_id.clone())),
            timed_annotation_id: Set(timed_annotation_id.clone()),
            panel_id: Set(panel_id),
        };

        annotation_panels::Entity::insert(record)
            .exec(client)
            .await?;
    }

    Ok(timed_annotation_id)
}

pub async fn add_many(
    dashboard_id: &str,
    org_id: &str,
    timed_annotations: Vec<TimedAnnotation>,
) -> Result<Vec<TimedAnnotation>, errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // Begin a transaction to ensure atomicity
    let txn = client.begin().await?;

    // Vector to store the inserted `TimedAnnotation` objects
    let mut inserted_annotations = Vec::new();

    for timed_annotation in timed_annotations {
        // Step 1: Insert the annotation into the `timed_annotations` table
        let annotation_id = ider::uuid(); // Generate a unique ID for the annotation
        let record = timed_annotations::ActiveModel {
            id: Set(annotation_id.clone()),
            dashboard_id: Set(dashboard_id.to_string()),
            start_time: Set(timed_annotation.start_time),
            end_time: Set(timed_annotation.end_time),
            title: Set(timed_annotation.title.clone()),
            text: Set(timed_annotation.text.clone()),
            tags: Set(timed_annotation.tags.clone()),
            created_at: Set(Utc::now().timestamp_micros()),
            org_id: Set(org_id.to_string()),
        };

        timed_annotations::Entity::insert(record)
            .exec(&txn) // Use the transaction
            .await?;

        // Step 2: Insert associated panels into the `annotation_panels` table
        for panel_id in &timed_annotation.panels {
            let panel_record = annotation_panels::ActiveModel {
                id: Set(ider::uuid()),
                timed_annotation_id: Set(annotation_id.clone()),
                panel_id: Set(panel_id.clone()),
            };

            annotation_panels::Entity::insert(panel_record)
                .exec(&txn) // Use the transaction
                .await?;
        }

        // Step 3: Construct the full `TimedAnnotation` object and add it to the list
        inserted_annotations.push(TimedAnnotation {
            annotation_id: Some(annotation_id),
            start_time: timed_annotation.start_time,
            end_time: timed_annotation.end_time,
            title: timed_annotation.title,
            text: timed_annotation.text,
            tags: timed_annotation.tags,
            panels: timed_annotation.panels,
        });
    }

    // Commit the transaction
    txn.commit().await?;

    // Return the list of inserted `TimedAnnotation` objects
    Ok(inserted_annotations)
}

pub async fn update(_dashboard_id: &str, _timed_annotation_id: &str) -> Result<(), errors::Error> {
    // TODO: update the timed_annotation table
    todo!()
}

pub async fn delete(dashboard_id: &str, timed_annotation_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // Step 1: Delete the annotation from `timed_annotations`
    let delete_result = timed_annotations::Entity::delete_many()
        .filter(timed_annotations::Column::Id.eq(timed_annotation_id))
        .filter(timed_annotations::Column::DashboardId.eq(dashboard_id))
        .exec(client)
        .await?;

    // Step 2: Check if the annotation was actually deleted
    if delete_result.rows_affected == 0 {
        return Err(errors::Error::DbError(errors::DbError::KeyNotExists(
            format!("Annotation with ID {} does not exist", timed_annotation_id),
        )));
    }

    Ok(())
}

pub async fn delete_many(
    dashboard_id: &str,
    timed_annotation_ids: &[String],
) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // Step 1: Build the condition to match multiple annotation IDs
    let mut condition = Condition::any(); // Use `Condition::any()` to OR multiple conditions
    for id in timed_annotation_ids {
        condition = condition.add(timed_annotations::Column::Id.eq(id));
    }

    // Step 2: Perform the batch deletion
    let delete_result = timed_annotations::Entity::delete_many()
        .filter(condition) // Match the IDs
        .filter(timed_annotations::Column::DashboardId.eq(dashboard_id)) // Ensure they belong to the same dashboard
        .exec(client)
        .await?;

    // Step 3: Check if any rows were deleted
    if delete_result.rows_affected == 0 {
        return Err(errors::Error::DbError(errors::DbError::KeyNotExists(
            "No matching annotations found for deletion".to_string(),
        )));
    }

    Ok(())
}
