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

use aes_siv::{siv::Aes256Siv, KeyInit};
use base64::{prelude::BASE64_STANDARD, Engine};
use config::get_config;
use once_cell::sync::Lazy;
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use super::{entity::cipher_keys::*, get_lock};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    CipherKey,
}

// DBKey to set cipher keys
pub const CIPHER_KEY_PREFIX: &str = "/cipher_keys/";
static MASTER_KEY: Lazy<Algorithm> = Lazy::new(|| {
    let config = get_config();
    // we currently only support one algorithm, so directly get key
    let key = match BASE64_STANDARD.decode(&config.encryption.master_key) {
        Ok(v) => v,
        Err(e) => {
            log::debug!("potential error in configuring master encryption for cipher table: {e}");
            log::info!("configuring cipher table master key as None");
            return Algorithm::None;
        }
    };
    match Aes256Siv::new_from_slice(&key) {
        Ok(_) => Algorithm::Aes256Siv(key),
        Err(e) => {
            log::debug!("potential error in configuring master encryption for cipher table: {e}");
            log::info!("configuring cipher table master key as None");
            Algorithm::None
        }
    }
});

enum Algorithm {
    Aes256Siv(Vec<u8>),
    None,
}

impl Algorithm {
    fn encrypt(&self, plaintext: &str) -> Result<String, errors::Error> {
        match self {
            Self::Aes256Siv(k) => {
                let mut c = Aes256Siv::new_from_slice(k).unwrap();
                c.encrypt([&[]], plaintext.as_bytes())
                    .map_err(|e| {
                        errors::Error::Message(format!(
                            "error encrypting data for cipher table {e}"
                        ))
                    })
                    .map(|v| BASE64_STANDARD.encode(&v))
            }
            Self::None => Ok(plaintext.to_owned()),
        }
    }
    fn decrypt(&self, encrypted: &str) -> Result<String, errors::Error> {
        match self {
            Self::Aes256Siv(k) => {
                let mut c = Aes256Siv::new_from_slice(k).unwrap();
                let v = BASE64_STANDARD.decode(encrypted).unwrap();
                c.decrypt([&[]], &v)
                    .map_err(|e| {
                        errors::Error::Message(format!(
                            "error decrypting data for cipher table {e}"
                        ))
                    })
                    .map(|v| String::from_utf8_lossy(&v).into_owned())
            }
            Self::None => Ok(encrypted.to_owned()),
        }
    }
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CipherKey => write!(f, "cipher_key"),
        }
    }
}

impl TryFrom<String> for EntryKind {
    type Error = errors::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "cipher_key" => Ok(Self::CipherKey),
            _ => Err(errors::Error::NotImplemented),
        }
    }
}

pub struct ListFilter {
    pub org: Option<String>,
    pub kind: Option<EntryKind>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CipherEntry {
    pub org: String,
    pub created_at: i64,
    pub created_by: String,
    pub name: String,
    pub data: String,
    pub kind: EntryKind,
}

impl TryInto<CipherEntry> for Model {
    type Error = errors::Error;
    fn try_into(self) -> Result<CipherEntry, Self::Error> {
        Ok(CipherEntry {
            org: self.org,
            created_at: self.created_at,
            created_by: self.created_by,
            kind: self.kind.try_into().unwrap(), // we can be fairly certain that this will not fail
            name: self.name,
            data: self.data,
        })
    }
}

pub async fn add(entry: CipherEntry) -> Result<(), errors::Error> {
    let encrypted = MASTER_KEY.encrypt(&entry.data)?;
    let record = ActiveModel {
        org: Set(entry.org),
        created_by: Set(entry.created_by),
        created_at: Set(entry.created_at),
        name: Set(entry.name),
        kind: Set(entry.kind.to_string()),
        data: Set(encrypted),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::insert(record).exec(client).await?;
    drop(_lock);

    Ok(())
}

pub async fn update(entry: CipherEntry) -> Result<(), errors::Error> {
    let encrypted = MASTER_KEY.encrypt(&entry.data)?;
    let record = ActiveModel {
        org: Set(entry.org),
        created_by: Set(entry.created_by),
        created_at: Set(entry.created_at),
        name: Set(entry.name),
        kind: Set(entry.kind.to_string()),
        data: Set(encrypted),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::update(record).exec(client).await?;
    drop(_lock);

    Ok(())
}

pub async fn remove(org: &str, kind: EntryKind, name: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Org.eq(org))
        .filter(Column::Kind.eq(kind.to_string()))
        .filter(Column::Name.eq(name))
        .exec(client)
        .await?;

    drop(_lock);

    Ok(())
}

pub async fn get_data(
    org: &str,
    kind: EntryKind,
    name: &str,
) -> Result<Option<String>, errors::Error> {
    let res = get(org, kind, name).await?;
    match res {
        Some(m) => MASTER_KEY.decrypt(&m.data).map(Some),
        None => Ok(None),
    }
}

async fn get(org: &str, kind: EntryKind, name: &str) -> Result<Option<Model>, errors::DbError> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::find()
        .filter(Column::Org.eq(org))
        .filter(Column::Name.eq(name))
        .filter(Column::Kind.eq(kind.to_string()))
        .into_model::<Model>()
        .one(client)
        .await
        .map_err(|e| errors::DbError::SeaORMError(e.to_string()))
}

pub async fn list_all(limit: Option<i64>) -> Result<Vec<CipherEntry>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedAt, Order::Desc);
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<Model>().all(client).await?;

    let records: Vec<CipherEntry> = records
        .into_iter()
        .map(<Model as TryInto<CipherEntry>>::try_into)
        .collect::<Result<Vec<_>, _>>()?;

    records
        .into_iter()
        .map(|mut c: CipherEntry| match MASTER_KEY.decrypt(&c.data) {
            Ok(d) => {
                c.data = d;
                Ok(c)
            }
            Err(e) => Err(e),
        })
        .collect()
}

pub async fn list_filtered(
    filter: ListFilter,
    limit: Option<i64>,
) -> Result<Vec<CipherEntry>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedAt, Order::Desc);
    if let Some(ref org) = filter.org {
        res = res.filter(Column::Org.eq(org));
    }
    if let Some(ref kind) = filter.kind {
        res = res.filter(Column::Kind.eq(kind.to_string()));
    }
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<Model>().all(client).await?;

    let records: Vec<CipherEntry> = records
        .into_iter()
        .map(<Model as TryInto<CipherEntry>>::try_into)
        .collect::<Result<Vec<_>, _>>()?;

    records
        .into_iter()
        .map(|mut c: CipherEntry| match MASTER_KEY.decrypt(&c.data) {
            Ok(d) => {
                c.data = d;
                Ok(c)
            }
            Err(e) => Err(e),
        })
        .collect()
}

pub async fn clear() -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many().exec(client).await?;

    Ok(())
}
