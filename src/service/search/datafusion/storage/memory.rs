use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use futures::{stream::BoxStream, StreamExt};
use object_store::MultipartId;
use object_store::{path::Path, GetResult, ListResult, ObjectMeta, ObjectStore, Result};
use std::ops::Range;
use thiserror::Error as ThisError;
use tokio::io::AsyncWrite;

use crate::infra::cache::file_data;

/// A specialized `Error` for in-memory object store-related errors
#[derive(ThisError, Debug)]
#[allow(missing_docs)]
enum Error {
    #[error("Out of range")]
    OutOfRange,

    #[error("Bad range")]
    BadRange,
}

impl From<Error> for object_store::Error {
    fn from(source: Error) -> Self {
        Self::Generic {
            store: "InMemory",
            source: Box::new(source),
        }
    }
}

/// In-memory storage suitable for testing or for opting out of using a cloud
/// storage provider.
#[derive(Debug, Default)]
pub struct InMemory {}

impl std::fmt::Display for InMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InMemory")
    }
}

#[async_trait]
impl ObjectStore for InMemory {
    async fn get(&self, location: &Path) -> Result<GetResult> {
        log::info!("get: {}", location);
        let data = self.get_bytes(location).await?;

        Ok(GetResult::Stream(
            futures::stream::once(async move { Ok(data) }).boxed(),
        ))
    }

    async fn get_range(&self, location: &Path, range: Range<usize>) -> Result<Bytes> {
        // log::info!("get_range: {}, {:?}", location, range);
        let data = self.get_bytes(location).await?;
        if range.end > data.len() {
            return Err(Error::OutOfRange.into());
        }
        if range.start > range.end {
            return Err(Error::BadRange.into());
        }
        Ok(data.slice(range))
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<usize>]) -> Result<Vec<Bytes>> {
        // log::info!("get_ranges: {}, {:?}", location, ranges);
        let data = self.get_bytes(location).await?;
        ranges
            .iter()
            .map(|range| {
                if range.end > data.len() {
                    return Err(Error::OutOfRange.into());
                }
                if range.start > range.end {
                    return Err(Error::BadRange.into());
                }
                Ok(data.slice(range.clone()))
            })
            .collect()
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        log::info!("head: {}", location);
        let last_modified = Utc::now();
        let bytes = self.get_bytes(location).await?;
        Ok(ObjectMeta {
            location: location.clone(),
            last_modified,
            size: bytes.len(),
        })
    }

    async fn list(&self, prefix: Option<&Path>) -> Result<BoxStream<'_, Result<ObjectMeta>>> {
        // log::info!("list: {:?}", prefix);
        let key = prefix.unwrap().to_string();
        let objects = super::file_list::get(&key).await.unwrap();
        let values = objects
            .iter()
            .map(|file| Ok(file.to_owned()))
            .collect::<Vec<Result<ObjectMeta>>>();
        Ok(futures::stream::iter(values).boxed())
    }

    /// The memory implementation returns all results, as opposed to the cloud
    /// versions which limit their results to 1k or more because of API
    /// limitations.
    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        log::info!("list_with_delimiter: {:?}", prefix);
        let key = prefix.unwrap().to_string();
        let objects = super::file_list::get(&key).await.unwrap();
        Ok(ListResult {
            objects,
            common_prefixes: vec![prefix.unwrap().clone()],
        })
    }

    async fn put(&self, location: &Path, _bytes: Bytes) -> Result<()> {
        log::error!("NotImplemented put: {}", location);
        Err(object_store::Error::NotImplemented {})
    }

    async fn put_multipart(
        &self,
        location: &Path,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        log::error!("NotImplemented put_multipart: {}", location);
        Err(object_store::Error::NotImplemented {})
    }

    async fn abort_multipart(&self, location: &Path, _multipart_id: &MultipartId) -> Result<()> {
        log::error!("NotImplemented abort_multipart: {}", location);
        Err(object_store::Error::NotImplemented {})
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        log::error!("NotImplemented delete: {}", location);
        Err(object_store::Error::NotImplemented {})
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        log::error!("NotImplemented copy: from {} to {}", from, to);
        Err(object_store::Error::NotImplemented {})
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        log::error!("NotImplemented copy_if_not_exists: from {} to {}", from, to);
        Err(object_store::Error::NotImplemented {})
    }
}

impl InMemory {
    /// Create new in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }

    async fn get_bytes(&self, location: &Path) -> Result<Bytes> {
        let file = location.to_string();
        let data = file_data::get(&file);
        tokio::task::yield_now().await;
        if data.is_ok() {
            return Ok(data.unwrap());
        }

        // log::info!("get_bytes from storage: {:?}", location);
        let data = match file_data::download(&file).await {
            Ok(data) => data,
            Err(e) => {
                log::info!("get_bytes from storage error: {:?}", e);
                return Err(object_store::Error::NotFound {
                    path: location.to_string(),
                    source: e.into(),
                });
            }
        };

        Ok(data)
    }
}
