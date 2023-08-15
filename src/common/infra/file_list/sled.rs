// Copyright 2022 Zinc Labs Inc. and Contributors
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

use async_trait::async_trait;
use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};

use crate::common::infra::{
    config::CONFIG,
    errors::{Error, Result},
};
use crate::common::meta::{
    common::{FileKey, FileMeta},
    stream::PartitionTimeLevel,
    StreamType,
};

lazy_static! {
    static ref CLIENT: ::sled::Db = connect();
}

pub async fn init() -> Result<()> {
    _ = CLIENT.clone().is_empty();
    Ok(())
}

pub fn connect() -> ::sled::Db {
    ::sled::open(format!("{}file_list.sled", CONFIG.common.data_cache_dir))
        .expect("sled db dir create failed")
}

pub struct SledFileList {}

impl SledFileList {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SledFileList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FileList for SledFileList {
    async fn add(&self, file: &str, meta: &FileMeta) -> Result<()> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");
        let client = CLIENT.clone();
        let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
        bucket.insert::<&str, Vec<u8>>(&file_name, meta.into())?;
        Ok(())
    }

    async fn remove(&self, file: &str) -> Result<()> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");
        let client = CLIENT.clone();
        let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
        bucket.remove::<&str>(&file_name)?;
        Ok(())
    }

    async fn batch_add(&self, files: &[FileKey]) -> Result<()> {
        let chunks = files.chunks(100);
        for files in chunks {
            let mut groups: ahash::AHashMap<String, Vec<FileKey>> = ahash::AHashMap::default();
            for file in files.iter() {
                let (stream_key, date_key, file_name) = super::parse_file_key_columns(&file.key)?;
                let entry = groups.entry(stream_key).or_default();
                entry.push(FileKey {
                    key: format!("{date_key}/{file_name}"),
                    meta: file.meta,
                    deleted: file.deleted,
                });
            }
            for (stream_key, files) in groups {
                let client = CLIENT.clone();
                let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
                let mut batch = sled::Batch::default();
                for file in files {
                    batch.insert::<&str, Vec<u8>>(&file.key, (&file.meta).into());
                }
                bucket.apply_batch(batch)?;
            }
        }
        Ok(())
    }

    async fn batch_remove(&self, files: &[String]) -> Result<()> {
        let chunks = files.chunks(100);
        for files in chunks {
            let mut groups: ahash::AHashMap<String, Vec<String>> = ahash::AHashMap::default();
            for file in files.iter() {
                let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
                let file = format!("{date_key}/{file_name}");
                let entry = groups.entry(stream_key).or_default();
                entry.push(file);
            }
            for (stream_key, files) in groups {
                let client = CLIENT.clone();
                let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
                let mut batch = sled::Batch::default();
                for file in files {
                    batch.remove::<&str>(&file);
                }
                bucket.apply_batch(batch)?;
            }
        }
        Ok(())
    }

    async fn get(&self, file: &str) -> Result<FileMeta> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");
        let client = CLIENT.clone();
        let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
        match bucket.get::<&str>(&file_name)? {
            Some(bytes) => Ok(FileMeta::try_from(bytes.as_ref())?),
            None => Err(Error::Message("file not found".to_string())),
        }
    }

    async fn list(&self) -> Result<Vec<(String, FileMeta)>> {
        let mut data = vec![];
        let client = CLIENT.clone();
        for (_, stream_key, items) in client.export() {
            let stream_key = String::from_utf8(stream_key.to_vec()).unwrap();
            let items = items
                .into_iter()
                .map(|v| {
                    let file_name = String::from_utf8(v.get(0).unwrap().to_vec()).unwrap();
                    let meta = FileMeta::try_from(v.get(1).unwrap().as_slice()).unwrap();
                    (format!("files/{stream_key}/{file_name}"), meta)
                })
                .collect::<Vec<_>>();
            data.extend(items);
        }
        Ok(data)
    }

    async fn query(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_level: PartitionTimeLevel,
        time_range: (i64, i64),
    ) -> Result<Vec<(String, FileMeta)>> {
        let (time_start, mut time_end) = time_range;
        if time_start == 0 {
            return Err(Error::Message(
                "Disallow empty time range query".to_string(),
            ));
        }
        if time_end == 0 {
            time_end = Utc::now().timestamp_micros();
        }

        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");
        let prefixes = generate_prefix(time_level, (time_start, time_end));
        let client = CLIENT.clone();
        let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
        let mut files = vec![];
        for prefix in prefixes {
            files.extend(
                bucket
                    .scan_prefix(&prefix)
                    .filter_map(|v| match v {
                        Err(e) => {
                            log::error!("sled scan prefix error: {}", e);
                            None
                        }
                        Ok((key, value)) => {
                            let file_name =
                                String::from_utf8(key.to_vec()).expect("sled file key parse error");
                            let meta = FileMeta::try_from(value.as_ref())
                                .expect("sled file meta parse error");
                            if (meta.min_ts >= time_start && meta.min_ts <= time_end)
                                || (meta.max_ts >= time_start && meta.max_ts <= time_end)
                            {
                                Some((format!("files/{stream_key}/{file_name}"), meta))
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<_>>(),
            );
        }
        Ok(files)
    }

    async fn contains(&self, file: &str) -> Result<bool> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");
        let client = CLIENT.clone();
        let bucket = client.open_tree(stream_key.as_bytes()).unwrap();
        Ok(bucket.contains_key::<&str>(&file_name)?)
    }

    async fn len(&self) -> usize {
        let client = CLIENT.clone();
        client.len()
    }

    async fn is_empty(&self) -> bool {
        let client = CLIENT.clone();
        client.is_empty()
    }

    async fn clear(&self) -> Result<()> {
        let client = CLIENT.clone();
        client.clear()?;
        Ok(())
    }
}

fn generate_prefix(time_level: PartitionTimeLevel, time_range: (i64, i64)) -> Vec<String> {
    let mut keys = Vec::new();
    let (time_min, time_max) = time_range;
    let time_min = Utc.timestamp_nanos(time_min * 1000);
    let time_max = Utc.timestamp_nanos(time_max * 1000);
    if time_min + Duration::hours(48) >= time_max {
        if time_level == PartitionTimeLevel::Daily {
            keys.push(time_min.format("%Y/%m/%d/00/").to_string());
        }
        // less than 48 hours, generate keys by hours
        let mut time_min = Utc
            .with_ymd_and_hms(
                time_min.year(),
                time_min.month(),
                time_min.day(),
                time_min.hour(),
                0,
                0,
            )
            .unwrap();
        while time_min <= time_max {
            keys.push(time_min.format("%Y/%m/%d/%H/").to_string());
            time_min += Duration::hours(1);
        }
    } else {
        // more than 48 hours, generate keys by days
        let mut time_min = Utc
            .with_ymd_and_hms(time_min.year(), time_min.month(), time_min.day(), 0, 0, 0)
            .unwrap();
        while time_min <= time_max {
            keys.push(time_min.format("%Y/%m/%d/").to_string());
            time_min += Duration::days(1);
        }
    }
    keys
}
