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

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use config::{get_config, meta::meta_store::MetaStore};
use hashbrown::HashMap;
use sea_orm::{DatabaseConnection, SqlxMySqlConnector, SqlxPostgresConnector, SqlxSqliteConnector};
use tokio::sync::{mpsc, OnceCell};

use crate::errors::{DbError, Error, Result};

pub mod etcd;
pub mod mysql;
pub mod nats;
pub mod postgres;
pub mod sqlite;

pub static NEED_WATCH: bool = true;
pub static NO_NEED_WATCH: bool = false;

static DEFAULT: OnceCell<Box<dyn Db>> = OnceCell::const_new();
static LOCAL_CACHE: OnceCell<Box<dyn Db>> = OnceCell::const_new();
static CLUSTER_COORDINATOR: OnceCell<Box<dyn Db>> = OnceCell::const_new();
static SUPER_CLUSTER: OnceCell<Box<dyn Db>> = OnceCell::const_new();

pub const SQLITE_STORE: &str = "sqlite";

pub static ORM_CLIENT: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn connect_to_orm() -> DatabaseConnection {
    match get_config().common.meta_store.as_str().into() {
        MetaStore::MySQL => {
            let pool = mysql::CLIENT.clone();
            SqlxMySqlConnector::from_sqlx_mysql_pool(pool)
        }
        MetaStore::PostgreSQL => {
            let pool = postgres::CLIENT.clone();
            SqlxPostgresConnector::from_sqlx_postgres_pool(pool)
        }
        _ => {
            let pool = { sqlite::CLIENT_RW.lock().await.clone() };
            SqlxSqliteConnector::from_sqlx_sqlite_pool(pool)
        }
    }
}

pub async fn get_db() -> &'static Box<dyn Db> {
    DEFAULT.get_or_init(default).await
}

pub async fn get_local_cache() -> &'static Box<dyn Db> {
    LOCAL_CACHE.get_or_init(init_local_cache).await
}

pub async fn get_coordinator() -> &'static Box<dyn Db> {
    CLUSTER_COORDINATOR
        .get_or_init(init_cluster_coordinator)
        .await
}

pub async fn get_super_cluster() -> &'static Box<dyn Db> {
    SUPER_CLUSTER.get_or_init(init_super_cluster).await
}

pub async fn init() -> Result<()> {
    etcd::init().await;
    create_table().await?;
    Ok(())
}

async fn default() -> Box<dyn Db> {
    let cfg = get_config();
    if !cfg.common.local_mode
        && (cfg.common.meta_store == "sled" || cfg.common.meta_store == "sqlite")
    {
        panic!("cluster mode is not supported for ZO_META_STORE=sqlite");
    }

    match cfg.common.meta_store.as_str().into() {
        MetaStore::Sqlite => Box::<sqlite::SqliteDb>::default(),
        MetaStore::Etcd => Box::<etcd::Etcd>::default(),
        MetaStore::Nats => Box::<nats::NatsDb>::default(),
        MetaStore::MySQL => Box::<mysql::MysqlDb>::default(),
        MetaStore::PostgreSQL => Box::<postgres::PostgresDb>::default(),
    }
}

async fn init_local_cache() -> Box<dyn Db> {
    Box::<sqlite::SqliteDb>::default()
}

async fn init_cluster_coordinator() -> Box<dyn Db> {
    let cfg = get_config();
    if cfg.common.local_mode {
        match cfg.common.meta_store.as_str().into() {
            MetaStore::Sqlite => Box::<sqlite::SqliteDb>::default(),
            _ => Box::<sqlite::SqliteDb>::default(),
        }
    } else {
        match cfg.common.cluster_coordinator.as_str().into() {
            MetaStore::Nats => Box::<nats::NatsDb>::default(),
            _ => Box::<etcd::Etcd>::default(),
        }
    }
}

pub async fn put_into_db_coordinator(
    key: &str,
    value: Bytes,
    need_watch: bool,
    start_dt: Option<i64>,
) -> Result<()> {
    let cfg = get_config();
    let value = if cfg.common.local_mode {
        value
    } else {
        Bytes::from("")
    };
    let cluster_coordinator = get_coordinator().await;
    cluster_coordinator
        .put(key, value, need_watch, start_dt)
        .await
}

pub async fn delete_from_db_coordinator(
    key: &str,
    with_prefix: bool,
    need_watch: bool,
    start_dt: Option<i64>,
) -> Result<()> {
    let cluster_coordinator = get_coordinator().await;
    cluster_coordinator
        .delete(key, with_prefix, need_watch, start_dt)
        .await
}

async fn init_super_cluster() -> Box<dyn Db> {
    if get_config().common.local_mode {
        panic!("super cluster is not supported in local mode");
    }
    Box::new(nats::NatsDb::super_cluster())
}

pub async fn create_table() -> Result<()> {
    // check db dir
    std::fs::create_dir_all(&get_config().common.data_db_dir)?;
    // create for meta store
    let cluster_coordinator = get_coordinator().await;
    cluster_coordinator.create_table().await?;
    let db = get_db().await;
    db.create_table().await?;
    Ok(())
}

pub type UpdateFn = dyn FnOnce(Option<Bytes>) -> Result<Option<(Option<Bytes>, Option<(String, Bytes, Option<i64>)>)>>
    + Send;

#[async_trait]
pub trait Db: Sync + Send + 'static {
    async fn create_table(&self) -> Result<()>;
    async fn stats(&self) -> Result<Stats>;
    async fn get(&self, key: &str) -> Result<Bytes>;
    async fn put(
        &self,
        key: &str,
        value: Bytes,
        need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()>;
    async fn get_for_update(
        &self,
        key: &str,
        need_watch: bool,
        start_dt: Option<i64>,
        update_fn: Box<UpdateFn>,
    ) -> Result<()>;
    async fn delete(
        &self,
        key: &str,
        with_prefix: bool,
        need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()>;

    /// Contrary to `delete`, this call won't fail if `key` is missing.
    async fn delete_if_exists(&self, key: &str, with_prefix: bool, need_watch: bool) -> Result<()> {
        match self.delete(key, with_prefix, need_watch, None).await {
            Ok(()) | Err(Error::DbError(DbError::KeyNotExists(_))) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn list(&self, prefix: &str) -> Result<HashMap<String, Bytes>>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn list_values(&self, prefix: &str) -> Result<Vec<Bytes>>;
    async fn list_values_by_start_dt(
        &self,
        prefix: &str,
        start_dt: Option<(i64, i64)>,
    ) -> Result<Vec<(i64, Bytes)>>;
    async fn count(&self, prefix: &str) -> Result<i64>;
    async fn watch(&self, prefix: &str) -> Result<Arc<mpsc::Receiver<Event>>>;
    async fn close(&self) -> Result<()>;
    async fn add_start_dt_column(&self) -> Result<()>;
}

pub fn parse_key(mut key: &str) -> (String, String, String) {
    let mut module = "".to_string();
    let mut key1 = "".to_string();
    let mut key2 = "".to_string();
    if key.starts_with('/') {
        key = &key[1..];
    }
    if key.is_empty() {
        return (module, key1, key2);
    }
    let columns = key.split('/').collect::<Vec<&str>>();
    match columns.len() {
        0 => {}
        1 => {
            module = columns[0].to_string();
        }
        2 => {
            module = columns[0].to_string();
            key1 = columns[1].to_string();
        }
        3 => {
            module = columns[0].to_string();
            key1 = columns[1].to_string();
            key2 = columns[2].to_string();
        }
        _ => {
            module = columns[0].to_string();
            key1 = columns[1].to_string();
            key2 = columns[2..].join("/").trim_end_matches('/').to_string();
        }
    }
    (module, key1, key2)
}

pub fn build_key(module: &str, key1: &str, key2: &str, start_dt: i64) -> String {
    if key1.is_empty() {
        format!("/{module}/")
    } else if key2.is_empty() {
        format!("/{module}/{key1}")
    } else if start_dt == 0 {
        format!("/{module}/{key1}/{key2}")
    } else {
        format!("/{module}/{key1}/{key2}/{start_dt}")
    }
}

#[derive(Debug, Default)]
pub struct Stats {
    pub bytes_len: i64,
    pub keys_count: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Put(EventData),
    Delete(EventData),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventData {
    pub key: String,
    pub value: Option<Bytes>,
    pub start_dt: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct MetaRecord {
    pub id: i64,
    pub module: String,
    pub key1: String,
    pub key2: String,
    pub start_dt: i64,
    pub value: String,
}

#[derive(Hash, Clone, Eq, PartialEq)]
struct DBIndex {
    name: String,
    table: String,
}

pub struct IndexStatement<'a> {
    pub idx_name: &'a str,
    pub table: &'a str,
    pub unique: bool,
    pub fields: &'a [&'a str],
}

impl<'a> IndexStatement<'a> {
    pub fn new(idx_name: &'a str, table: &'a str, unique: bool, fields: &'a [&'a str]) -> Self {
        Self {
            idx_name,
            table,
            unique,
            fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put() {
        create_table().await.unwrap();
        let db = get_db().await;
        db.put("/foo/put/bar", Bytes::from("hello"), false, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get() {
        create_table().await.unwrap();
        let db = get_db().await;
        let hello = Bytes::from("hello");
        db.put("/foo/get/bar", hello.clone(), false, None)
            .await
            .unwrap();
        assert_eq!(db.get("/foo/get/bar").await.unwrap(), hello);
    }

    #[tokio::test]
    async fn test_delete() {
        create_table().await.unwrap();
        let db = get_db().await;
        let hello = Bytes::from("hello");

        db.put("/foo/del/bar1", hello.clone(), false, None)
            .await
            .unwrap();
        db.put("/foo/del/bar2", hello.clone(), false, None)
            .await
            .unwrap();
        db.put("/foo/del/bar3", hello.clone(), false, None)
            .await
            .unwrap();
        db.delete("/foo/del/bar1", false, false, None)
            .await
            .unwrap();
        assert!(db.delete("/foo/del/bar4", false, false, None).await.is_ok());
        db.delete("/foo/del/", true, false, None).await.unwrap();

        db.put("/foo/del/bar1", hello.clone(), false, None)
            .await
            .unwrap();
        db.put("/foo/del/bar2", hello.clone(), false, None)
            .await
            .unwrap();
        db.put("/foo/del/bar3", hello, false, None).await.unwrap();
        assert_eq!(db.list_keys("/foo/del/").await.unwrap().len(), 3);
        assert_eq!(db.list_values("/foo/del/").await.unwrap().len(), 3);
    }
}
