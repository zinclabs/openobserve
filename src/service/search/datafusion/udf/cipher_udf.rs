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

use arrow::array::{ArrayRef, StringArray};
use config::utils::str;
use datafusion::{
    arrow::datatypes::DataType,
    common::cast::as_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarFunctionImplementation, ScalarUDF, Volatility},
    prelude::create_udf,
    scalar::ScalarValue,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

use crate::cipher::registry::REGISTRY;

/// The name of the decrypt UDF given to DataFusion.
pub(crate) const DECRYPT_UDF_NAME: &str = "decrypt";
/// The name of the decrypt UDF given to DataFusion.
pub(crate) const ENCRYPT_UDF_NAME: &str = "encrypt";

/// implementation of decrypt
pub(crate) static DECRYPT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        DECRYPT_UDF_NAME,
        // expects two arguments : field and key_name
        vec![DataType::Utf8, DataType::Utf8],
        // returns string
        DataType::Utf8,
        Volatility::Stable,
        decrypt(),
    )
});

/// implementation of decrypt
pub(crate) static ENCRYPT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ENCRYPT_UDF_NAME,
        // expects two arguments : field and key_name
        vec![DataType::Utf8, DataType::Utf8],
        // returns string
        DataType::Utf8,
        Volatility::Stable,
        encrypt(),
    )
});

/// decrypt function
fn decrypt() -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 2 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "decrypt requires tow params : decrypt(field_name, key_name)".to_string(),
                ),
                None,
            ));
        }

        let key = match &args[1] {
            ColumnarValue::Scalar(ScalarValue::Utf8(Some(s))) => s.to_owned(),
            _ => {
                return Err(DataFusionError::SQL(
                    ParserError::ParserError(
                        "second argument to decrypt must be a key-name string".to_string(),
                    ),
                    None,
                ))
            }
        };

        let args = ColumnarValue::values_to_arrays(args)?;

        let values = as_string_array(&args[0]).map_err(|_| {
            DataFusionError::SQL(
                ParserError::ParserError(
                    "first argument to decrypt must be a string type column".to_string(),
                ),
                None,
            )
        })?;

        // NOTE!!! : the {} block is important as it will drop read lock
        // if we take a read lock outside of block scope, it might not be dropped
        let mut cipher = {
            match REGISTRY.read().get_key(&key) {
                None => {
                    let key_name = match key.split_once(":") {
                        Some((_, n)) => n,
                        None => &key,
                    };
                    return Err(DataFusionError::Execution(format!(
                        "key with name {} not found",
                        key_name
                    )));
                }
                Some(k) => k,
            }
        };

        let mut err_count = 0;
        let mut last_error = None;
        let ret = values
            .iter()
            .map(|v| {
                v.map(|s| match cipher.decrypt(s) {
                    Ok(v) => v,
                    Err(e) => {
                        err_count += 1;
                        last_error = Some(e);
                        s.to_owned()
                    }
                })
            })
            .collect::<StringArray>();

        if let Some(e) = last_error {
            log::info!("encountered some errors while decrypting, total count {err_count}, last error : {e}");
        }

        Ok(ColumnarValue::from(Arc::new(ret) as ArrayRef))
    })
}

/// decrypt function
fn encrypt() -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 2 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "encrypt requires tow params : encrypt(field_name, key_name)".to_string(),
                ),
                None,
            ));
        }

        let key = match &args[1] {
            ColumnarValue::Scalar(ScalarValue::Utf8(Some(s))) => s.to_owned(),
            _ => {
                return Err(DataFusionError::SQL(
                    ParserError::ParserError(
                        "second argument to encrypt must be a key-name string".to_string(),
                    ),
                    None,
                ))
            }
        };

        let args = ColumnarValue::values_to_arrays(args)?;

        let values = as_string_array(&args[0]).map_err(|_| {
            DataFusionError::SQL(
                ParserError::ParserError(
                    "first argument to encrypt must be a string type column".to_string(),
                ),
                None,
            )
        })?;

        // NOTE!!! : the {} block is important as it will drop read lock
        // if we take a read lock outside of block scope, it might not be dropped
        let mut cipher = {
            match REGISTRY.read().get_key(&key) {
                None => {
                    let key_name = match key.split_once(":") {
                        Some((_, n)) => n,
                        None => &key,
                    };
                    return Err(DataFusionError::Execution(format!(
                        "key with name {} not found",
                        key_name
                    )));
                }
                Some(k) => k,
            }
        };

        let mut err_count = 0;
        let mut last_error = None;
        let ret = values
            .iter()
            .map(|v| {
                v.map(|s| match cipher.encrypt(s) {
                    Ok(v) => v,
                    Err(e) => {
                        err_count += 1;
                        last_error = Some(e);
                        s.to_owned()
                    }
                })
            })
            .collect::<StringArray>();

        if let Some(e) = last_error {
            log::info!("encountered some errors while decrypting, total count {err_count}, last error : {e}");
        }

        Ok(ColumnarValue::from(Arc::new(ret) as ArrayRef))
    })
}
