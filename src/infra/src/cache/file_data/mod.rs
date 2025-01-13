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

pub mod disk;
pub mod memory;

use std::{collections::VecDeque, ops::Range};

use hashbrown::HashSet;
use hashlink::lru_cache::LruCache;

const INITIAL_CACHE_SIZE: usize = 128;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CacheType {
    Disk,
    Memory,
    None,
}

enum CacheStrategy {
    Lru(LruCache<String, usize>),
    Fifo((VecDeque<(String, usize)>, HashSet<String>)),
}

impl CacheStrategy {
    fn new(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "lru" => CacheStrategy::Lru(LruCache::new_unbounded()),
            "fifo" => CacheStrategy::Fifo((
                VecDeque::with_capacity(INITIAL_CACHE_SIZE),
                HashSet::with_capacity(INITIAL_CACHE_SIZE),
            )),
            _ => CacheStrategy::Lru(LruCache::new_unbounded()),
        }
    }

    fn insert(&mut self, key: String, value: usize) {
        match self {
            CacheStrategy::Lru(cache) => {
                cache.insert(key, value);
            }
            CacheStrategy::Fifo((queue, set)) => {
                set.insert(key.clone());
                queue.push_back((key, value));
            }
        }
    }

    fn remove(&mut self) -> Option<(String, usize)> {
        match self {
            CacheStrategy::Lru(cache) => cache.remove_lru(),
            CacheStrategy::Fifo((queue, set)) => {
                if queue.is_empty() {
                    return None;
                }
                let (key, size) = queue.pop_front().unwrap();
                set.remove(&key);
                Some((key, size))
            }
        }
    }

    fn contains_key(&self, key: &str) -> bool {
        match self {
            CacheStrategy::Lru(cache) => cache.contains_key(key),
            CacheStrategy::Fifo((_, set)) => set.contains(key),
        }
    }

    fn len(&self) -> usize {
        match self {
            CacheStrategy::Lru(cache) => cache.len(),
            CacheStrategy::Fifo((queue, _)) => queue.len(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            CacheStrategy::Lru(cache) => cache.is_empty(),
            CacheStrategy::Fifo((queue, _)) => queue.is_empty(),
        }
    }

    fn remove_key(&mut self, key: &str) -> Option<(String, usize)> {
        match self {
            CacheStrategy::Lru(cache) => cache.remove_entry(key),
            CacheStrategy::Fifo((queue, set)) => {
                if queue.is_empty() {
                    return None;
                }
                let mut index = 0;
                while index < queue.len() {
                    if queue[index].0 == key {
                        let (k, v) = queue.remove(index).unwrap();
                        set.remove(&k);
                        return Some((k, v));
                    }
                    index += 1;
                }
                None
            }
        }
    }
}

pub async fn init() -> Result<(), anyhow::Error> {
    disk::init().await?;
    memory::init().await?;
    Ok(())
}

pub async fn download(trace_id: &str, file: &str) -> Result<(), anyhow::Error> {
    let cfg = config::get_config();
    if cfg.memory_cache.enabled {
        memory::download(trace_id, file).await
    } else if cfg.disk_cache.enabled {
        disk::download(trace_id, file).await
    } else {
        Ok(())
    }
}

/// set the data to the cache
///
/// store the data to the memory cache or disk cache
pub async fn set(trace_id: &str, key: &str, data: bytes::Bytes) -> Result<(), anyhow::Error> {
    let cfg = config::get_config();
    // set the data to the memory cache
    if cfg.memory_cache.enabled {
        memory::set(trace_id, key, data).await
    } else if cfg.disk_cache.enabled {
        disk::set(trace_id, key, data).await
    } else {
        Ok(())
    }
}

pub async fn get(
    _trace_id: &str,
    file: &str,
    range: Option<Range<usize>>,
) -> object_store::Result<bytes::Bytes> {
    get_opts(_trace_id, file, range, true).await
}

pub async fn get_opts(
    _trace_id: &str,
    file: &str,
    range: Option<Range<usize>>,
    remote: bool,
) -> object_store::Result<bytes::Bytes> {
    let cfg = config::get_config();
    // get from memory cache
    if cfg.memory_cache.enabled {
        if let Some(v) = memory::get(file, range.clone()).await {
            return Ok(v);
        }
    }
    // get from disk cache
    if cfg.disk_cache.enabled {
        if let Some(v) = disk::get(file, range.clone()).await {
            return Ok(v);
        }
    }
    // get from storage
    if remote {
        return match range {
            Some(r) => crate::storage::get_range(file, r).await,
            None => crate::storage::get(file).await,
        };
    }

    Err(object_store::Error::NotFound {
        path: file.to_string(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, file)),
    })
}

pub async fn get_size(_trace_id: &str, file: &str) -> object_store::Result<usize> {
    get_size_opts(_trace_id, file, true).await
}

pub async fn get_size_opts(
    _trace_id: &str,
    file: &str,
    remote: bool,
) -> object_store::Result<usize> {
    let cfg = config::get_config();
    // get from memory cache
    if cfg.memory_cache.enabled {
        if let Some(v) = memory::get_size(file).await {
            return Ok(v);
        }
    }
    // get from disk cache
    if cfg.disk_cache.enabled {
        if let Some(v) = disk::get_size(file).await {
            return Ok(v);
        }
    }
    // get from storage
    if remote {
        let meta = crate::storage::head(file).await?;
        return Ok(meta.size);
    }

    Err(object_store::Error::NotFound {
        path: file.to_string(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, file)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_miss() {
        let mut cache = CacheStrategy::new("lru");
        let key1 = "a";
        let key2 = "b";
        cache.insert(key1.to_string(), 1);
        cache.insert(key2.to_string(), 2);
        cache.contains_key(key1);
        cache.remove(); // contains_key() does not mark the key1 as used -> removed
        assert!(!cache.contains_key(key1));
        assert!(cache.contains_key(key2));
    }

    #[test]
    fn test_fifo_cache_miss() {
        let mut cache = CacheStrategy::new("fifo");
        let key1 = "a";
        let key2 = "b";
        cache.insert(key1.to_string(), 1);
        cache.insert(key2.to_string(), 2);
        cache.contains_key(key1);
        cache.remove();
        assert!(!cache.contains_key(key1));
        assert!(cache.contains_key(key2));
    }
}
