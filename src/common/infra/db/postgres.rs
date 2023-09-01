// Copyright 2023 Zinc Labs Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use ahash::HashMap;
use async_trait::async_trait;
use bytes::Bytes;
use once_cell::sync::Lazy;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, Pool, Postgres,
};
use std::{str::FromStr, sync::Arc};
use tokio::{sync::mpsc, task::JoinHandle, time};

use crate::common::infra::{cluster, config::CONFIG, errors::*};

pub(crate) static CLIENT: Lazy<Pool<Postgres>> = Lazy::new(connect);

fn connect() -> Pool<Postgres> {
    let db_opts = PgConnectOptions::from_str(&CONFIG.common.meta_store_postgres_dsn)
        .expect("postgres connect options create failed")
        .disable_statement_logging();

    let pool_opts = PgPoolOptions::new();
    let pool_opts = pool_opts.min_connections(CONFIG.limit.cpu_num as u32);
    let pool_opts = pool_opts.max_connections(CONFIG.limit.query_thread_num as u32);
    pool_opts.connect_lazy_with(db_opts)
}

pub struct PostgresDb {}

impl PostgresDb {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PostgresDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Db for PostgresDb {
    async fn stats(&self) -> Result<super::Stats> {
        let pool = CLIENT.clone();
        let keys_count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*)::BIGINT as num FROM meta;"#)
            .fetch_one(&pool)
            .await
            .unwrap_or_default();
        let bytes_len: i64 =   sqlx::query_scalar(r#"SELECT (page_count * page_size)::BIGINT as size FROM pragma_page_count(), pragma_page_size();"#)
        .fetch_one(&pool)
        .await.unwrap_or_default();
        Ok(super::Stats {
            bytes_len,
            keys_count,
        })
    }

    async fn get(&self, key: &str) -> Result<Bytes> {
        let (module, key_1, key_2) = parse_key(key);
        let pool = CLIENT.clone();
        let value: Vec<u8> = match sqlx::query_scalar(
            r#"SELECT value FROM meta WHERE module = $1 AND key1 = $2 AND key2 = $3;"#,
        )
        .bind(module)
        .bind(key_1)
        .bind(key_2)
        .fetch_one(&pool)
        .await
        {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::from(DbError::KeyNotExists(key.to_string())));
            }
        };
        Ok(Bytes::from(value))
    }

    async fn put(&self, key: &str, value: Bytes, need_watch: bool) -> Result<()> {
        let (module, key_1, key_2) = parse_key(key);
        let pool = CLIENT.clone();

        let mut tx = pool.begin().await?;
        if let Err(e) = sqlx::query(
            r#"INSERT INTO meta (module, key_1, key_2, value) VALUES ($1, $2, $3, '');"#,
        )
        .bind(&module)
        .bind(&key_1)
        .bind(&key_2)
        .execute(&mut *tx)
        .await
        {
            log::error!("[POSTGRES] insert meta error: {}, key: {}", e, key);
        }

        sqlx::query(r#"UPDATE meta SET value=$4 WHERE module = $1 AND key1 = $2 AND key2 = $3;"#)
            .bind(&module)
            .bind(&key_1)
            .bind(&key_2)
            .bind(value.as_ref())
            .execute(&pool)
            .await?;

        // TODO: event watch
        if !need_watch {
            return Ok(());
        }

        Ok(())
    }

    async fn delete(&self, _key: &str, _with_prefix: bool, need_watch: bool) -> Result<()> {
        // TODO: event watch
        if !need_watch {
            return Ok(());
        }

        Ok(())
    }

    async fn list(&self, _prefix: &str) -> Result<HashMap<String, Bytes>> {
        let result = HashMap::default();
        Ok(result)
    }

    async fn list_keys(&self, _prefix: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn list_values(&self, _prefix: &str) -> Result<Vec<Bytes>> {
        Ok(Vec::new())
    }

    async fn count(&self, _prefix: &str) -> Result<i64> {
        Ok(0)
    }

    async fn watch(&self, _prefix: &str) -> Result<Arc<mpsc::Receiver<super::Event>>> {
        let (tx, rx) = mpsc::channel(1024);
        let _task: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                if cluster::is_offline() {
                    break;
                }
                tx.send(super::Event::Empty).await.unwrap();
                time::sleep(time::Duration::from_secs(10)).await;
            }
            Ok(())
        });

        Ok(Arc::new(rx))
    }
}

fn parse_key(mut key: &str) -> (String, String, String) {
    let mut module = "".to_string();
    let mut key_1 = "".to_string();
    let mut key_2 = "".to_string();
    if key.starts_with('/') {
        key = &key[1..];
    }
    if key.is_empty() {
        return (module, key_1, key_2);
    }
    let columns = key.split('/').collect::<Vec<&str>>();
    match columns.len() {
        0 => {}
        1 => {
            module = columns[0].to_string();
        }
        2 => {
            module = columns[0].to_string();
            key_1 = columns[1].to_string();
        }
        3 => {
            module = columns[0].to_string();
            key_1 = columns[1].to_string();
            key_2 = columns[2].to_string();
        }
        _ => {
            module = columns[0].to_string();
            key_1 = columns[1].to_string();
            key_2 = columns[2..].join("/");
        }
    }
    (module, key_1, key_2)
}

pub async fn create_table() -> Result<()> {
    let pool = CLIENT.clone();
    // create table
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS meta
(
    id      INTEGER  not null primary key autoincrement,
    module  VARCHAR  not null,
    key1    VARCHAR not null,
    key2    VARCHAR not null,
    value   BLOB not null
);
        "#,
    )
    .execute(&pool)
    .await?;
    // create table index
    sqlx::query(
        r#"
CREATE INDEX IF NOT EXISTS meta_module_idx on meta (module);
CREATE INDEX IF NOT EXISTS meta_module_key1_idx on meta (module, key1);
CREATE UNIQUE INDEX IF NOT EXISTS meta_module_key2_idx on meta (module, key1, key2);
        "#,
    )
    .execute(&pool)
    .await?;
    Ok(())
}
