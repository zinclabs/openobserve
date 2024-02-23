// Copyright 2023 Zinc Labs Inc.
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

use ::config::CONFIG;

pub mod cluster;
pub mod config;
pub mod ofga;
pub mod wal;

pub async fn init() -> Result<(), anyhow::Error> {
    wal::init().await?;
    // because of asynchronous, we need to wait for a while
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // check imcomplete work group
    #[cfg(feature = "enterprise")]
    o2_enterprise::enterprise::search::queue::clean(CONFIG.limit.query_timeout as i64).await?;

    Ok(())
}
