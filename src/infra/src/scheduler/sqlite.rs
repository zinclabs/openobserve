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

use async_trait::async_trait;
use chrono::Duration;
use config::utils::json;
use sqlx::{Pool, Row, Sqlite};

use super::{get_scheduler_max_retries, Trigger, TriggerModule, TriggerStatus, TRIGGERS_KEY};
use crate::{
    db::{
        self,
        sqlite::{create_index, CLIENT_RO, CLIENT_RW},
        IndexStatement,
    },
    errors::{DbError, Error, Result},
};

pub struct SqliteScheduler {}

impl SqliteScheduler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqliteScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Scheduler for SqliteScheduler {
    /// Creates the Scheduled Jobs table
    async fn create_table(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS scheduled_jobs
(
    id           INTEGER not null primary key autoincrement,
    org          VARCHAR(100) not null,
    module       INT not null,
    module_key   VARCHAR(256) not null,
    is_realtime  BOOLEAN default false not null,
    is_silenced  BOOLEAN default false not null,
    status       INT not null,
    start_time   BIGINT,
    end_time     BIGINT,
    retries      INT not null,
    next_run_at  BIGINT not null,
    created_at   TIMESTAMP default CURRENT_TIMESTAMP,
    data         TEXT not null
);
            "#,
        )
        .execute(&*client)
        .await?;

        // Add data column for old version <= 0.10.9
        add_data_column(&client).await?;
        Ok(())
    }

    async fn create_table_index(&self) -> Result<()> {
        create_index(IndexStatement::new(
            "scheduled_jobs_key_idx",
            "scheduled_jobs",
            false,
            &["module_key"],
        ))
        .await?;
        create_index(IndexStatement::new(
            "scheduled_jobs_org_key_idx",
            "scheduled_jobs",
            false,
            &["org", "module_key"],
        ))
        .await?;
        create_index(IndexStatement::new(
            "scheduled_jobs_org_module_key_idx",
            "scheduled_jobs",
            true,
            &["org", "module", "module_key"],
        ))
        .await?;

        Ok(())
    }

    /// The count of jobs for the given module (Report/Alert etc.)
    async fn len_module(&self, module: TriggerModule) -> usize {
        let pool = CLIENT_RO.clone();
        let ret = match sqlx::query(
            r#"
SELECT COUNT(*) as num FROM scheduled_jobs WHERE module = $1;"#,
        )
        .bind(module)
        .fetch_one(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[SQLITE] triggers len error: {}", e);
                return 0;
            }
        };
        match ret.try_get::<i64, &str>("num") {
            Ok(v) => v as usize,
            _ => 0,
        }
    }

    /// Pushes a Trigger job into the queue
    async fn push(&self, trigger: Trigger) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;

        if let Err(e) = sqlx::query(
            r#"
INSERT INTO scheduled_jobs (org, module, module_key, is_realtime, is_silenced, status, retries, next_run_at, start_time, end_time, data)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ON CONFLICT DO NOTHING;
        "#,
        )
        .bind(&trigger.org)
        .bind(&trigger.module)
        .bind(&trigger.module_key)
        .bind(trigger.is_realtime)
        .bind(trigger.is_silenced)
        .bind(&trigger.status)
        .bind(0)
        .bind(trigger.next_run_at)
        .bind(0)
        .bind(0)
        .bind(&trigger.data)
        .execute(&mut *tx)
        .await
        {
            if let Err(e) = tx.rollback().await {
                log::error!("[SQLITE] rollback push scheduled_jobs error: {}", e);
            }
            return Err(e.into());
        }

        if let Err(e) = tx.commit().await {
            log::error!("[SQLITE] commit push scheduled_jobs error: {}", e);
            return Err(e.into());
        }

        // release lock
        drop(client);

        // For now, only send realtime alert triggers
        if trigger.module == TriggerModule::Alert && trigger.is_realtime {
            let key = format!(
                "{TRIGGERS_KEY}{}/{}/{}",
                trigger.module, &trigger.org, &trigger.module_key
            );

            // TODO: For sqlite cluster coordinator, the alert triggers are put
            // into the sqlite meta database to send watch events. Hence, there is a
            // redundancy of alert triggers stored both in scheduled_jobs and meta
            // tables. How to remove this redundancy?
            let cluster_coordinator = db::get_coordinator().await;
            cluster_coordinator
                .put(&key, json::to_vec(&trigger).unwrap().into(), true, None)
                .await?;
        }
        Ok(())
    }

    /// Deletes the Trigger job matching the given parameters
    async fn delete(&self, org: &str, module: TriggerModule, key: &str) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(
            r#"DELETE FROM scheduled_jobs WHERE org = $1 AND module_key = $2 AND module = $3;"#,
        )
        .bind(org)
        .bind(key)
        .bind(&module)
        .execute(&*client)
        .await?;

        drop(client);

        // For now, only send alert triggers
        if module == TriggerModule::Alert {
            // For status update of triggers, we don't need to send put events
            // to cluster coordinator for now as it only changes the status and retries
            // fields of scheduled jobs and not anything else
            let key = format!("{TRIGGERS_KEY}{}/{}/{}", module, org, key);
            let cluster_coordinator = db::get_coordinator().await;
            cluster_coordinator.delete(&key, false, true, None).await?;
        }
        Ok(())
    }

    /// Updates the status of the Trigger job
    async fn update_status(
        &self,
        org: &str,
        module: TriggerModule,
        key: &str,
        status: TriggerStatus,
        retries: i32,
        data: Option<&str>,
    ) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let query = match data {
            Some(data) => {
                sqlx::query(
                    r#"UPDATE scheduled_jobs SET status = $1, retries = $2, data = $3 WHERE org = $4 AND module_key = $5 AND module = $6;"#,
                )
                .bind(status)
                .bind(retries)
                .bind(data)
                .bind(org)
                .bind(key)
                .bind(&module)
            },
            None => {
                sqlx::query(
                    r#"UPDATE scheduled_jobs SET status = $1, retries = $2 WHERE org = $3 AND module_key = $4 AND module = $5;"#,
                )
                .bind(status)
                .bind(retries)
                .bind(org)
                .bind(key)
                .bind(&module)
            },
        };
        query.execute(&*client).await?;

        drop(client);

        // For status update of triggers, we don't need to send put events
        // to cluster coordinator for now as it only changes the status and retries
        // fields of scheduled jobs and not anything else
        Ok(())
    }

    async fn update_trigger(&self, trigger: Trigger) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(
            r#"UPDATE scheduled_jobs
SET status = $1, retries = $2, next_run_at = $3, is_realtime = $4, is_silenced = $5, data = $6
WHERE org = $7 AND module_key = $8 AND module = $9;"#,
        )
        .bind(&trigger.status)
        .bind(trigger.retries)
        .bind(trigger.next_run_at)
        .bind(trigger.is_realtime)
        .bind(trigger.is_silenced)
        .bind(&trigger.data)
        .bind(&trigger.org)
        .bind(&trigger.module_key)
        .bind(&trigger.module)
        .execute(&*client)
        .await?;

        // release lock
        drop(client);

        // For now, only send alert triggers
        if trigger.module == TriggerModule::Alert && trigger.is_realtime {
            let key = format!(
                "{TRIGGERS_KEY}{}/{}/{}",
                trigger.module, &trigger.org, &trigger.module_key
            );
            let cluster_coordinator = db::get_coordinator().await;
            cluster_coordinator
                .put(&key, json::to_vec(&trigger).unwrap().into(), true, None)
                .await?;
        }
        Ok(())
    }

    /// Returns the Trigger jobs with "Waiting" status.
    /// Steps:
    /// - Lock the Sqlite client for read-write
    /// - Read the records with status "Waiting", oldest createdAt, next_run_at <= now and limit =
    ///   `concurrency`
    /// - Changes their statuses from "Waiting" to "Processing"
    /// - Commits as a single transaction
    /// - Returns the Trigger jobs
    async fn pull(
        &self,
        concurrency: i64,
        alert_timeout: i64,
        report_timeout: i64,
    ) -> Result<Vec<Trigger>> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;

        let (include_max, mut max_retries) = get_scheduler_max_retries();
        if include_max {
            max_retries += 1;
        }
        let now = chrono::Utc::now().timestamp_micros();
        let report_max_time = now
            + Duration::try_seconds(report_timeout)
                .unwrap()
                .num_microseconds()
                .unwrap();
        let alert_max_time = now
            + Duration::try_seconds(alert_timeout)
                .unwrap()
                .num_microseconds()
                .unwrap();
        let query = r#"UPDATE scheduled_jobs
SET status = $1, start_time = $2,
    end_time = CASE
        WHEN module = $3 THEN $4
        ELSE $5
    END
WHERE id IN (
    SELECT id
    FROM scheduled_jobs
    WHERE status = $6 AND next_run_at <= $7 AND retries < $8 AND NOT (is_realtime = $9 AND is_silenced = $10)
    ORDER BY next_run_at
    LIMIT $11
)
RETURNING *;"#;

        let jobs: Vec<Trigger> = sqlx::query_as::<_, Trigger>(query)
            .bind(TriggerStatus::Processing)
            .bind(now)
            .bind(TriggerModule::Alert)
            .bind(alert_max_time)
            .bind(report_max_time)
            .bind(TriggerStatus::Waiting)
            .bind(now)
            .bind(max_retries)
            .bind(true)
            .bind(false)
            .bind(concurrency)
            .fetch_all(&*client)
            .await?;
        Ok(jobs)
    }

    async fn get(&self, org: &str, module: TriggerModule, key: &str) -> Result<Trigger> {
        let pool = CLIENT_RO.clone();
        let query = r#"
SELECT * FROM scheduled_jobs
WHERE org = $1 AND module = $2 AND module_key = $3;"#;
        let job = match sqlx::query_as::<_, Trigger>(query)
            .bind(org)
            .bind(module.clone())
            .bind(key)
            .fetch_one(&pool)
            .await
        {
            Ok(job) => job,
            Err(_) => {
                return Err(Error::from(DbError::KeyNotExists(format!(
                    "{org}/{}/{key}",
                    module
                ))));
            }
        };
        Ok(job)
    }

    async fn list(&self, module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
        let client = CLIENT_RO.clone();
        let jobs: Vec<Trigger> = if let Some(module) = module {
            let query = r#"SELECT * FROM scheduled_jobs WHERE module = $1 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(module)
                .fetch_all(&client)
                .await?
        } else {
            let query = r#"SELECT * FROM scheduled_jobs ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .fetch_all(&client)
                .await?
        };
        Ok(jobs)
    }

    /// List all the jobs for the given module and organization
    async fn list_by_org(&self, org: &str, module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
        let client = CLIENT_RO.clone();
        let jobs: Vec<Trigger> = if let Some(module) = module {
            let query =
                r#"SELECT * FROM scheduled_jobs WHERE org = $1 AND module = $2 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(org)
                .bind(module)
                .fetch_all(&client)
                .await?
        } else {
            let query = r#"SELECT * FROM scheduled_jobs WHERE org = $1 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(org)
                .fetch_all(&client)
                .await?
        };
        Ok(jobs)
    }

    /// Background job that frequently (30 secs interval) cleans "Completed" jobs or jobs with
    /// retries >= threshold set through environment
    async fn clean_complete(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let (include_max, mut max_retries) = get_scheduler_max_retries();
        if include_max {
            max_retries += 1;
        }
        // Since alert scheduled_jobs contain last_satisfied_at field, we should not delete them
        sqlx::query(
            r#"DELETE FROM scheduled_jobs WHERE (status = $1 OR retries >= $2) AND module != $3;"#,
        )
        .bind(TriggerStatus::Completed)
        .bind(max_retries)
        .bind(TriggerModule::Alert)
        .execute(&*client)
        .await?;
        Ok(())
    }

    /// Background job that watches for timeout of a job
    /// Steps:
    /// - Select all the records with status = "Processing"
    /// - calculate the current timestamp and difference from `start_time` of each record
    /// - Get the record ids with difference more than the given timeout
    /// - Update their status back to "Waiting" and increase their "retries" by 1
    async fn watch_timeout(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let now = chrono::Utc::now().timestamp_micros();
        sqlx::query(
            r#"UPDATE scheduled_jobs
SET status = $1, retries = retries + 1
WHERE status = $2 AND end_time <= $3;
                "#,
        )
        .bind(TriggerStatus::Waiting)
        .bind(TriggerStatus::Processing)
        .bind(now)
        .execute(&*client)
        .await?;
        Ok(())
    }

    async fn len(&self) -> usize {
        let pool = CLIENT_RO.clone();
        let ret = match sqlx::query(
            r#"
SELECT COUNT(*) as num FROM scheduled_jobs;"#,
        )
        .fetch_one(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[SQLITE] triggers len error: {}", e);
                return 0;
            }
        };
        match ret.try_get::<i64, &str>("num") {
            Ok(v) => v as usize,
            _ => 0,
        }
    }

    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    async fn clear(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        match sqlx::query(r#"DELETE FROM scheduled_jobs;"#)
            .execute(&*client)
            .await
        {
            Ok(_) => log::info!("[SCHEDULER] scheduled_jobs table cleared"),
            Err(e) => log::error!("[SQLITE] error clearing scheduled_jobs table: {}", e),
        }

        Ok(())
    }
}

async fn add_data_column(client: &Pool<Sqlite>) -> Result<()> {
    // Attempt to add the column, ignoring the error if the column already exists
    if let Err(e) =
        sqlx::query(r#"ALTER TABLE scheduled_jobs ADD COLUMN data TEXT NOT NULL DEFAULT '';"#)
            .execute(client)
            .await
    {
        // Check if the error is about the duplicate column
        if !e.to_string().contains("duplicate column name") {
            // If the error is not about the duplicate column, return it
            return Err(e.into());
        }
    }

    Ok(())
}
