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

use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use bytes::Bytes;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tantivy::{directory::OwnedBytes, Directory, ReloadPolicy};

use super::{caching_directory::CachingDirectory, FOOTER_CACHE};

const FOOTER_CACHE_VERSION: u32 = 1;
const FOOTER_VERSION_LEN: usize = 4;
const FOOTER_OFFSET_LEN: usize = 8;

pub(crate) struct FooterCache {
    data: RwLock<HashMap<PathBuf, HashMap<Range<usize>, OwnedBytes>>>,
}

impl FooterCache {
    pub(crate) fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) fn get_slice(&self, path: &Path, byte_range: Range<usize>) -> Option<OwnedBytes> {
        self.data
            .read()
            .get(path)
            .and_then(|map| map.get(&byte_range).cloned())
    }

    pub(crate) fn put_slice(&self, path: PathBuf, byte_range: Range<usize>, bytes: OwnedBytes) {
        self.data
            .write()
            .entry(path)
            .or_default()
            .insert(byte_range, bytes);
    }

    #[allow(dead_code)]
    pub(crate) fn file_num(&self) -> usize {
        self.data.read().len()
    }

    pub(crate) fn to_bytes(&self) -> tantivy::Result<Bytes> {
        let mut buf = Vec::new();
        let r = self.data.read();
        let mut metadata = FooterCacheMeta::new();
        // write data
        for (path, slice_data) in r.iter() {
            for (range, bytes) in slice_data.iter() {
                let offset = buf.len();
                buf.extend_from_slice(bytes);
                metadata.push(path, offset, range);
            }
        }
        // write metadata
        let offset = buf.len() as u64;
        let meta_bytes = serde_json::to_vec(&metadata).unwrap();
        buf.extend_from_slice(&meta_bytes);
        // write footer offset
        buf.extend_from_slice(&offset.to_le_bytes()[..]);
        // write footer version
        buf.extend_from_slice(&FOOTER_CACHE_VERSION.to_le_bytes()[..]);
        Ok(buf.into())
    }

    pub(crate) fn from_bytes(bytes: OwnedBytes) -> tantivy::Result<Self> {
        // parse version
        let range = bytes.len() - FOOTER_VERSION_LEN..bytes.len();
        let footer_version = u32::from_le_bytes(bytes.slice(range).to_vec().try_into().unwrap());
        if footer_version != FOOTER_CACHE_VERSION {
            return Err(tantivy::TantivyError::InvalidArgument(format!(
                "Invalid footer version: {}",
                footer_version
            )));
        }
        // parse footer offset
        let range =
            bytes.len() - FOOTER_OFFSET_LEN - FOOTER_VERSION_LEN..bytes.len() - FOOTER_VERSION_LEN;
        let footer_offset = u64::from_le_bytes(bytes.slice(range).to_vec().try_into().unwrap());
        // parse metadata
        let range = footer_offset as usize..(bytes.len() - FOOTER_OFFSET_LEN - FOOTER_VERSION_LEN);
        let metadata: FooterCacheMeta = serde_json::from_slice(&bytes.slice(range)).unwrap();
        // parse footer data
        let mut data = HashMap::new();
        for (path, items) in metadata.files.iter() {
            let mut slice_data = HashMap::new();
            for item in items.iter() {
                let range = item.start as usize..(item.start + item.len) as usize;
                let data = bytes.slice(item.offset as usize..(item.offset + item.len) as usize);
                slice_data.insert(range, data);
            }
            data.insert(PathBuf::from(path), slice_data);
        }
        Ok(Self {
            data: RwLock::new(data),
        })
    }

    pub(crate) async fn from_directory(source: Arc<dyn Directory>) -> tantivy::Result<Self> {
        let path = std::path::Path::new(FOOTER_CACHE);
        let file = source.get_file_handle(path)?;
        let data = file.read_bytes_async(0..file.len()).await?;
        Self::from_bytes(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FooterCacheMeta {
    files: HashMap<String, Vec<FooterCacheMetaItem>>,
}

impl FooterCacheMeta {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn push(&mut self, path: &Path, offset: usize, range: &Range<usize>) {
        self.files
            .entry(path.to_string_lossy().to_string())
            .or_default()
            .push(FooterCacheMetaItem {
                offset: offset as u64,
                start: range.start as u64,
                len: (range.end - range.start) as u64,
            });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FooterCacheMetaItem {
    offset: u64, // offset in the buffer file
    start: u64,  // range start
    len: u64,    // range lenth
}

pub(crate) fn build_footer_cache<D: Directory>(directory: Arc<D>) -> tantivy::Result<bytes::Bytes> {
    let cache_dir = CachingDirectory::new(directory);
    let index = tantivy::Index::open(cache_dir.clone())?;
    let schema = index.schema();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    let searcher = reader.searcher();
    for (field, field_entry) in schema.fields() {
        if !field_entry.is_indexed() {
            continue;
        }
        for reader in searcher.segment_readers() {
            let _inv_idx = reader.inverted_index(field)?;
        }
    }

    let buf = cache_dir.cacher().to_bytes()?;
    Ok(buf)
}
