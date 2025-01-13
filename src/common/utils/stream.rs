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

use std::io::{Error, ErrorKind};

use actix_web::HttpResponse;
use arrow::array::{Int64Array, RecordBatch};
use config::{
    get_config,
    meta::{
        stream::{FileMeta, StreamType},
        user::{User, UserRole},
    },
    FILE_EXT_JSON,
};

use crate::service::users;

#[inline(always)]
pub fn stream_type_query_param_error() -> Result<HttpResponse, Error> {
    Err(Error::new(
        ErrorKind::Other,
        "only 'type' query param with value 'logs' or 'metrics' allowed",
    ))
}

#[inline(always)]
pub fn increment_stream_file_num_v1(file_name: &str) -> u32 {
    let last_file_suffix_stream = &file_name.rfind('_').unwrap() + 1;
    let last_file_suffix =
        &file_name[last_file_suffix_stream..file_name.len() - FILE_EXT_JSON.len()];
    last_file_suffix.parse::<u32>().unwrap() + 1
}

#[inline(always)]
pub fn get_stream_file_num_v1(file_name: &str) -> u32 {
    let last_file_suffix_stream = &file_name.rfind('_').unwrap() + 1;
    let last_file_suffix =
        &file_name[last_file_suffix_stream..file_name.len() - FILE_EXT_JSON.len()];
    last_file_suffix.parse::<u32>().unwrap()
}

#[inline(always)]
pub fn get_file_name_v1(org_id: &str, stream_name: &str, suffix: u32) -> String {
    // creates file name like
    // "./data/openobserve/olympics/olympics#2022#09#13#13_1.json"
    format!(
        "{}{}/{}/{}/{}_{}{}",
        get_config().common.data_wal_dir,
        org_id,
        StreamType::Logs,
        stream_name,
        stream_name,
        suffix,
        FILE_EXT_JSON
    )
}

pub async fn populate_file_meta(
    batches: &[&RecordBatch],
    file_meta: &mut FileMeta,
    min_field: Option<&str>,
    max_field: Option<&str>,
) -> Result<(), anyhow::Error> {
    if batches.is_empty() {
        return Ok(());
    }
    let cfg = get_config();
    let min_field = min_field.unwrap_or_else(|| cfg.common.column_timestamp.as_str());
    let max_field = max_field.unwrap_or_else(|| cfg.common.column_timestamp.as_str());

    let total = batches.iter().map(|batch| batch.num_rows()).sum::<usize>();
    let mut min_val = i64::MAX;
    let mut max_val = 0;
    for batch in batches.iter() {
        let num_row = batch.num_rows();
        let Some(min_field) = batch.column_by_name(min_field) else {
            return Err(anyhow::anyhow!("No min_field found: {}", min_field));
        };
        let Some(max_field) = batch.column_by_name(max_field) else {
            return Err(anyhow::anyhow!("No max_field found: {}", max_field));
        };
        let min_col = min_field.as_any().downcast_ref::<Int64Array>().unwrap();
        let max_col = max_field.as_any().downcast_ref::<Int64Array>().unwrap();
        for i in 0..num_row {
            let val = min_col.value(i);
            if val < min_val {
                min_val = val;
            }
            let val = max_col.value(i);
            if val > max_val {
                max_val = val;
            }
        }
    }
    if min_val == i64::MAX {
        min_val = 0;
    }

    file_meta.min_ts = min_val;
    file_meta.max_ts = max_val;
    file_meta.records = total as i64;
    Ok(())
}

pub async fn get_settings_max_query_range(
    stream_max_query_range: i64,
    org_id: &str,
    user_id: Option<&str>,
) -> i64 {
    if user_id.is_none() {
        return stream_max_query_range;
    }

    if let Some(user) = users::get_user(Some(org_id), user_id.unwrap()).await {
        return get_max_query_range_if_sa(stream_max_query_range, &user);
    }

    stream_max_query_range
}

pub fn get_max_query_range_if_sa(stream_max_query_range: i64, user: &User) -> i64 {
    log::debug!("get_max_query_range_if_sa stream_max_query_range: {stream_max_query_range}, user_role: {:?}", user.role);
    if user.role == UserRole::ServiceAccount {
        let max_query_range_sa = get_config().limit.max_query_range_for_sa;
        return if max_query_range_sa > 0 {
            std::cmp::min(stream_max_query_range, max_query_range_sa)
        } else {
            stream_max_query_range
        };
    }
    stream_max_query_range
}

/// Get the maximum query range for a list of streams in hours
pub async fn get_max_query_range(
    stream_names: &[String],
    org_id: &str,
    user_id: &str,
    stream_type: StreamType,
) -> i64 {
    let user = users::get_user(Some(org_id), user_id).await;

    futures::future::join_all(
        stream_names
            .iter()
            .map(|stream_name| infra::schema::get_settings(org_id, stream_name, stream_type)),
    )
    .await
    .into_iter()
    .filter_map(|settings| {
        settings.map(|s| {
            if let Some(user) = &user {
                get_max_query_range_if_sa(s.max_query_range, user)
            } else {
                s.max_query_range
            }
        })
    })
    .max()
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datafusion::arrow::{
        array::StringArray,
        datatypes::{DataType, Field, Schema},
    };

    use super::*;

    #[test]
    fn test_increment_stream_file_num_v1() {
        let suffix_nums = [1, 9, 11, 78, 100, 234, 546];

        for &suffix in &suffix_nums {
            let new_suffix = increment_stream_file_num_v1(&format!(
                "./data/openobserve/WAL/nexus/logs/olympics/1663064862606912_{}.json",
                suffix
            ));
            assert_eq!(new_suffix as usize, suffix + 1);
        }
    }

    #[test]
    fn test_get_file_name_v1() {
        let file_key = get_file_name_v1("nexus", "Olympics", 2);
        assert!(file_key
            .as_str()
            .ends_with("/wal/nexus/logs/Olympics/Olympics_2.json"));
    }

    #[test]
    fn test_get_stream_file_num_v1() {
        let file_key =
            get_stream_file_num_v1("./data/openobserve/WAL/logs/nexus/Olympics/Olympics_2.json");
        assert_eq!(file_key, 2);
    }

    #[tokio::test]
    async fn test_populate_file_meta() {
        // define a schema.
        let val: i64 = 1666093521151350;
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("pod_id", DataType::Int64, false),
            Field::new("_timestamp", DataType::Int64, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["a", "b", "c", "d"])),
                Arc::new(Int64Array::from(vec![1, 2, 1, 2])),
                Arc::new(Int64Array::from(vec![val - 100, val - 10, val - 90, val])),
            ],
        )
        .unwrap();
        // let file_name = path.file_name();
        let mut file_meta = FileMeta {
            min_ts: 0,
            max_ts: 0,
            records: 0,
            original_size: 1000,
            compressed_size: 700,
            flattened: false,
            index_size: 0,
        };
        populate_file_meta(&[&batch], &mut file_meta, None, None)
            .await
            .unwrap();
        assert_eq!(file_meta.records, 4);
        assert_eq!(file_meta.min_ts, val - 100);
    }

    #[tokio::test]
    async fn test_populate_file_meta_with_custom_field() {
        // define a schema.
        let val: i64 = 1666093521151350;
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("pod_id", DataType::Int64, false),
            Field::new("time", DataType::Int64, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["a", "b", "c", "d"])),
                Arc::new(Int64Array::from(vec![1, 2, 1, 2])),
                Arc::new(Int64Array::from(vec![val - 100, val - 10, val - 90, val])),
            ],
        )
        .unwrap();
        // let file_name = path.file_name();
        let mut file_meta = FileMeta {
            min_ts: 0,
            max_ts: 0,
            records: 0,
            original_size: 1000,
            compressed_size: 700,
            flattened: false,
            index_size: 0,
        };
        populate_file_meta(&[&batch], &mut file_meta, Some("time"), Some("time"))
            .await
            .unwrap();
        assert_eq!(file_meta.records, 4);
        assert_eq!(file_meta.min_ts, val - 100);
    }

    #[test]
    fn test_stream_type_query_param_error() {
        let res = stream_type_query_param_error();
        assert!(res.is_err());
    }
}
