// Copyright 2024 Zinc Labs Inc.
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

use infra::table::cipher::{self, ListFilter};
use o2_enterprise::enterprise::cipher::CipherData;

use crate::cipher::registry::REGISTRY;

pub async fn run() -> Result<(), anyhow::Error> {
    log::info!("retrieving and caching cipher keys in memory");
    let filter = ListFilter {
        org: None,
        kind: Some(cipher::EntryKind::CipherKey),
    };

    let data_list = cipher::list_filtered(filter, None).await?;

    let data_list: Result<Vec<_>, serde_json::Error> = data_list
        .into_iter()
        .map(|e| {
            let cdata: Result<CipherData, _> = serde_json::from_str(&e.data);
            cdata.map(|d| (format!("{}:{}", e.org, e.name), d))
        })
        .collect();
    let data_list = data_list?;

    let mut keys = Vec::with_capacity(data_list.len());

    for (name, data) in data_list {
        let key = match data.get_key().await {
            Ok(k) => k,
            Err(e) => {
                // we continue here, so the successful keys can be used
                log::warn!("error in retrieving key {name}  : {e}");
                continue;
            }
        };
        keys.push((name, key));
    }
    {
        let mut reg = REGISTRY.write();
        for (name, key) in keys {
            reg.add_key(name, Box::new(key));
        }
    }
    log::info!("successfully cached cipher keys in memory");
    Ok(())
}
