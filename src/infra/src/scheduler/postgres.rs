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
use bytes::Bytes;
use chrono::Duration;
use config::metrics::DB_QUERY_NUMS;
use sqlx::Row;

use super::{get_scheduler_max_retries, Trigger, TriggerModule, TriggerStatus, TRIGGERS_KEY};
use crate::{
    db::{
        self,
        postgres::{create_index, CLIENT},
        IndexStatement,
    },
    errors::{DbError, Error, Result},
};

pub struct PostgresScheduler {}

impl PostgresScheduler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PostgresScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Scheduler for PostgresScheduler {
    /// Creates the Scheduled Jobs table
    async fn create_table(&self) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["create", "scheduled_jobs"])
            .inc();
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS scheduled_jobs
(
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
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
        .execute(&pool)
        .await?;

        // create start_dt column for old version <= 0.9.2
        DB_QUERY_NUMS
            .with_label_values(&["select", "information_schema.columns"])
            .inc();
        let has_data_column = sqlx::query_scalar::<_,i64>("SELECT count(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name='scheduled_jobs' AND column_name='data';")
            .fetch_one(&pool)
            .await?;
        if has_data_column == 0 {
            add_data_column().await?;
        }
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "scheduled_jobs"])
            .inc();
        let ret = match sqlx::query(
            r#"
SELECT COUNT(*)::BIGINT AS num FROM scheduled_jobs WHERE module = $1;"#,
        )
        .bind(module)
        .fetch_one(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[POSTGRES] triggers len error: {}", e);
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
        // let db = db::get_db().await;
        let pool = CLIENT.clone();
        let mut tx = pool.begin().await?;
        DB_QUERY_NUMS
            .with_label_values(&["insert", "scheduled_jobs"])
            .inc();
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
                log::error!("[POSTGRES] rollback push scheduled_jobs error: {}", e);
            }
            return Err(e.into());
        }

        if let Err(e) = tx.commit().await {
            log::error!("[POSTGRES] commit push scheduled_jobs error: {}", e);
            return Err(e.into());
        }

        // For now, only send realtime alert triggers
        if trigger.module == TriggerModule::Alert && trigger.is_realtime {
            let key = format!(
                "{TRIGGERS_KEY}{}/{}/{}",
                trigger.module, &trigger.org, &trigger.module_key
            );
            let cluster_coordinator = db::get_coordinator().await;
            cluster_coordinator
                .put(&key, Bytes::from(""), true, None)
                .await?;
        }
        Ok(())
    }

    /// Deletes the Trigger job matching the given parameters
    async fn delete(&self, org: &str, module: TriggerModule, key: &str) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["delete", "scheduled_jobs"])
            .inc();
        sqlx::query(
            r#"DELETE FROM scheduled_jobs WHERE org = $1 AND module_key = $2 AND module = $3;"#,
        )
        .bind(org)
        .bind(key)
        .bind(&module)
        .execute(&pool)
        .await?;

        // For now, only send alert triggers events
        if module == TriggerModule::Alert {
            // It will send event even if the alert is not realtime alert.
            // But that is okay, for non-realtime alerts, since the triggers are not
            // present in the cache at all, it will just do nothing.
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "scheduled_jobs"])
            .inc();
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
            }
            None => {
                sqlx::query(
                    r#"UPDATE scheduled_jobs SET status = $1, retries = $2 WHERE org = $3 AND module_key = $4 AND module = $5;"#,
                )
                .bind(status)
                .bind(retries)
                .bind(org)
                .bind(key)
                .bind(&module)
            }
        };
        query.execute(&pool).await?;

        // For status update of triggers, we don't need to send put events
        // to cluster coordinator for now as it only changes the status and retries
        // fields of scheduled jobs and not anything else
        Ok(())
    }

    async fn update_trigger(&self, trigger: Trigger) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "scheduled_jobs"])
            .inc();
        sqlx::query(
            r#"UPDATE scheduled_jobs
SET status = $1, retries = $2, next_run_at = $3, is_realtime = $4, is_silenced = $5, data = $6
WHERE org = $7 AND module_key = $8 AND module = $9;"#,
        )
        .bind(trigger.status)
        .bind(trigger.retries)
        .bind(trigger.next_run_at)
        .bind(trigger.is_realtime)
        .bind(trigger.is_silenced)
        .bind(&trigger.data)
        .bind(&trigger.org)
        .bind(&trigger.module_key)
        .bind(&trigger.module)
        .execute(&pool)
        .await?;

        // For now, only send realtime alert triggers
        if trigger.module == TriggerModule::Alert && trigger.is_realtime {
            let key = format!(
                "{TRIGGERS_KEY}{}/{}/{}",
                trigger.module, &trigger.org, &trigger.module_key
            );
            let cluster_coordinator = db::get_coordinator().await;
            cluster_coordinator
                .put(&key, Bytes::from(""), true, None)
                .await?;
        }
        Ok(())
    }

    /// Returns the Trigger jobs with "Waiting" status.
    /// Steps:
    /// - Read the records with status "Waiting", oldest createdAt, next_run_at <= now and limit =
    ///   `concurrency`
    /// - Skip locked rows and lock the read rows with "FOR UPDATE SKIP LOCKED"
    /// - Changes their statuses from "Waiting" to "Processing"
    /// - Commits as a single transaction
    /// - Returns the Trigger jobs
    async fn pull(
        &self,
        concurrency: i64,
        alert_timeout: i64,
        report_timeout: i64,
    ) -> Result<Vec<Trigger>> {
        let pool = CLIENT.clone();
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
        DB_QUERY_NUMS
            .with_label_values(&["update", "scheduled_jobs"])
            .inc();
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
    FOR UPDATE
)
RETURNING *;"#;

        let mut tx = pool.begin().await?;
        let jobs: Vec<Trigger> = match sqlx::query_as::<_, Trigger>(query)
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
            .fetch_all(&mut *tx)
            .await
        {
            Ok(jobs) => jobs,
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback pull scheduled_jobs error: {}", e);
                }
                return Err(e.into());
            }
        };

        if let Err(e) = tx.commit().await {
            log::error!("[POSTGRES] commit pull scheduled_jobs error: {}", e);
            return Err(e.into());
        }
        Ok(jobs)
    }

    async fn get(&self, org: &str, module: TriggerModule, key: &str) -> Result<Trigger> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "scheduled_jobs"])
            .inc();
        let query = r#"
SELECT * FROM scheduled_jobs
WHERE org = $1 AND module = $2 AND module_key = $3;"#;
        let job = match sqlx::query_as::<_, Trigger>(query)
            .bind(org)
            .bind(&module)
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "scheduled_jobs"])
            .inc();
        let jobs: Vec<Trigger> = if let Some(module) = module {
            let query = r#"SELECT * FROM scheduled_jobs WHERE module = $1 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(module)
                .fetch_all(&pool)
                .await?
        } else {
            let query = r#"SELECT * FROM scheduled_jobs ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query).fetch_all(&pool).await?
        };
        Ok(jobs)
    }

    /// List all the jobs for the given module and organization
    async fn list_by_org(&self, org: &str, module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "scheduled_jobs"])
            .inc();
        let jobs: Vec<Trigger> = if let Some(module) = module {
            let query =
                r#"SELECT * FROM scheduled_jobs WHERE org = $1 AND module = $2 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(org)
                .bind(module)
                .fetch_all(&pool)
                .await?
        } else {
            let query = r#"SELECT * FROM scheduled_jobs WHERE org = $1 ORDER BY id;"#;
            sqlx::query_as::<_, Trigger>(query)
                .bind(org)
                .fetch_all(&pool)
                .await?
        };
        Ok(jobs)
    }

    /// Background job that frequently (30 secs interval) cleans "Completed" jobs or jobs with
    /// retries >= threshold set through environment
    async fn clean_complete(&self) -> Result<()> {
        let pool = CLIENT.clone();
        let (include_max, mut max_retries) = get_scheduler_max_retries();
        if include_max {
            max_retries += 1;
        }
        DB_QUERY_NUMS
            .with_label_values(&["delete", "scheduled_jobs"])
            .inc();
        // Since alert scheduled_jobs contain last_satisfied_at field, we should not delete them
        sqlx::query(
            r#"DELETE FROM scheduled_jobs WHERE (status = $1 OR retries >= $2) AND module != $3;"#,
        )
        .bind(TriggerStatus::Completed)
        .bind(max_retries)
        .bind(TriggerModule::Alert)
        .execute(&pool)
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "scheduled_jobs"])
            .inc();
        let now = chrono::Utc::now().timestamp_micros();
        let res = sqlx::query(
            r#"UPDATE scheduled_jobs
SET status = $1, retries = retries + 1
WHERE status = $2 AND end_time <= $3;
                "#,
        )
        .bind(TriggerStatus::Waiting)
        .bind(TriggerStatus::Processing)
        .bind(now)
        .execute(&pool)
        .await?;
        log::debug!(
            "[SCHEDULER] watch_timeout for scheduler updated {} rows",
            res.rows_affected()
        );
        Ok(())
    }

    async fn len(&self) -> usize {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "scheduled_jobs"])
            .inc();
        let ret = match sqlx::query(
            r#"
SELECT COUNT(*)::BIGINT AS num FROM scheduled_jobs;"#,
        )
        .fetch_one(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[POSTGRES] triggers len error: {}", e);
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["delete", "scheduled_jobs"])
            .inc();
        match sqlx::query(r#"DELETE FROM scheduled_jobs;"#)
            .execute(&pool)
            .await
        {
            Ok(_) => log::info!("[SCHEDULER] scheduled_jobs table cleared"),
            Err(e) => log::error!("[POSTGRES] error clearing scheduled_jobs table: {}", e),
        }

        Ok(())
    }
}

async fn add_data_column() -> Result<()> {
    log::info!("[POSTGRES] Adding data column to scheduled_jobs table");
    let pool = CLIENT.clone();
    DB_QUERY_NUMS
        .with_label_values(&["alter", "scheduled_jobs"])
        .inc();
    if let Err(e) = sqlx::query(
        r#"ALTER TABLE scheduled_jobs ADD COLUMN IF NOT EXISTS data TEXT NOT NULL DEFAULT '';"#,
    )
    .execute(&pool)
    .await
    {
        log::error!("[POSTGRES] Error in adding column data: {}", e);
        return Err(e.into());
    }
    Ok(())
}
