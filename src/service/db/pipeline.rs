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

// use std::sync::Arc;

use std::sync::Arc;

use bytes::Bytes;
use config::meta::{
    pipeline::{components::PipelineSource, Pipeline},
    stream::StreamParams,
};
use infra::{
    db,
    pipeline::{self as infra_pipeline},
};
use once_cell::sync::Lazy;

use crate::{
    common::infra::config::STREAM_EXECUTABLE_PIPELINES,
    service::pipeline::batch_execution::ExecutablePipeline,
};

const PIPELINES_WATCH_PREFIX: &str = "/pipelines/";

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    // internal
    #[error("InfraError# {0}")]
    InfraError(#[from] infra::errors::Error),
    // not found
    #[error("Pipeline with ID {0} not found.")]
    NotFound(String),
    // conflict
    #[error("Pipeline with ID {0} modified by someone else. Please refresh.")]
    Modified(String),
    // bad request
    #[error("A realtime pipeline with same source stream already exists")]
    StreamInUse,
    #[error("Invalid pipeline {0}")]
    InvalidPipeline(String),
    #[error("Invalid DerivedStream config: {0}")]
    InvalidDerivedStream(String),
    #[error("Error deleting previous DerivedStream: {0}")]
    DeleteDerivedStream(String),
}

/// Stores a new pipeline to database.
///
/// Pipeline validation should be handled by the caller.
pub async fn set(pipeline: &Pipeline) -> Result<(), PipelineError> {
    infra_pipeline::put(pipeline).await?;

    // save to cache if realtime pipeline
    if let PipelineSource::Realtime(stream_params) = &pipeline.source {
        if pipeline.enabled {
            update_cache(stream_params, PipelineTableEvent::Add).await;
        }
    }

    Ok(())
}

/// Updates a pipeline entry with the sane values.
///
/// Pipeline validation should be handled by the caller.
pub async fn update(
    pipeline: &Pipeline,
    prev_source_stream: Option<StreamParams>,
) -> Result<(), PipelineError> {
    infra_pipeline::update(pipeline).await?;

    if let Some(prev_stream_params) = prev_source_stream {
        update_cache(&prev_stream_params, PipelineTableEvent::Remove).await;
    }

    // save to cache if realtime pipeline
    if let PipelineSource::Realtime(stream_params) = &pipeline.source {
        let db_event = if pipeline.enabled {
            PipelineTableEvent::Add
        } else {
            PipelineTableEvent::Remove
        };
        update_cache(stream_params, db_event).await;
    }

    Ok(())
}

/// Returns all streams with existing pipelines.
pub async fn list_streams_with_pipeline(org: &str) -> Result<Vec<StreamParams>, PipelineError> {
    Ok(infra_pipeline::list_streams_with_pipeline(org).await?)
}

/// Transform the initialized and enabled pipeline into ExecutablePipeline struct that's ready for
/// batch processing records.
///
/// Used for pipeline execution.
pub async fn get_executable_pipeline(stream_params: &StreamParams) -> Option<ExecutablePipeline> {
    if let Some(exec_pl) = STREAM_EXECUTABLE_PIPELINES.read().await.get(stream_params) {
        return Some(exec_pl.clone());
    }
    match get_by_stream(stream_params).await {
        Some(pl) if pl.enabled => match ExecutablePipeline::new(&pl).await {
            Ok(exec_pl) => {
                let mut stream_exec_pl_cache = STREAM_EXECUTABLE_PIPELINES.write().await;
                stream_exec_pl_cache.insert(stream_params.to_owned(), exec_pl.clone());
                drop(stream_exec_pl_cache);
                Some(exec_pl)
            }
            Err(e) => {
                log::error!(
                    "[Pipeline]: failed to initialize ExecutablePipeline from Pipeline read from database, {}",
                    e
                );
                None
            }
        },
        _ => None,
    }
}

/// Returns the pipeline by id.
///
/// Used to get the pipeline associated with the ID when scheduled job is ran.
pub async fn get_by_stream(stream_params: &StreamParams) -> Option<Pipeline> {
    infra_pipeline::get_by_stream(stream_params).await.ok()
}

/// Returns the pipeline by id.
///
/// Used to get the pipeline associated with the ID when scheduled job is ran.
pub async fn get_by_id(pipeline_id: &str) -> Result<Pipeline, PipelineError> {
    Ok(infra_pipeline::get_by_id(pipeline_id).await?)
}

/// Finds the pipeline with the same source
///
/// Used to validate if a duplicate pipeline exists.
pub async fn get_with_same_source_stream(pipeline: &Pipeline) -> Result<Pipeline, PipelineError> {
    Ok(infra_pipeline::get_with_same_source_stream(pipeline).await?)
}

/// Lists all pipelines across all orgs.
pub async fn list() -> Result<Vec<Pipeline>, PipelineError> {
    Ok(infra_pipeline::list().await?)
}

/// Lists all pipelines for a given organization.
pub async fn list_by_org(org: &str) -> Result<Vec<Pipeline>, PipelineError> {
    Ok(infra_pipeline::list_by_org(org).await?)
}

/// Deletes a pipeline by ID.
pub async fn delete(pipeline_id: &str) -> Result<(), PipelineError> {
    let pipeline = infra_pipeline::delete(pipeline_id).await?;

    // remove from cache if realtime pipeline
    if let PipelineSource::Realtime(stream_params) = &pipeline.source {
        update_cache(stream_params, PipelineTableEvent::Remove).await;
    }

    Ok(())
}

/// Preload all enabled pipelines into the cache at startup.
pub async fn cache() -> Result<(), anyhow::Error> {
    let pipelines = list().await?;
    if pipelines
        .iter()
        .any(|pl| pl.enabled && pl.num_of_func() > 0)
        && !config::get_config().common.mmdb_disable_download
    {
        log::info!("[PIPELINE:CACHE] waiting mmdb data to be available");
        Lazy::force(&crate::job::MMDB_INIT_NOTIFIER)
            .notified()
            .await;
        log::info!("[PIPELINE:CACHE] done waiting");
    }
    let mut stream_exec_pl = STREAM_EXECUTABLE_PIPELINES.write().await;
    for pipeline in pipelines.into_iter() {
        if pipeline.enabled {
            if let PipelineSource::Realtime(stream_params) = &pipeline.source {
                match ExecutablePipeline::new(&pipeline).await {
                    Err(e) => {
                        log::error!(
                            "[Pipeline] error initializing ExecutablePipeline from pipeline {}/{}. {}. Not cached",
                            pipeline.org,
                            pipeline.id,
                            e
                        )
                    }
                    Ok(exec_pl) => {
                        stream_exec_pl.insert(stream_params.clone(), exec_pl);
                    }
                };
            }
        }
    }
    log::info!("[Pipeline] Cached with len: {}", stream_exec_pl.len());
    Ok(())
}

/// Update STREAM_PIPELINES cache for realtime pipelines
async fn update_cache(stream_params: &StreamParams, event: PipelineTableEvent) {
    let cache_key = get_cache_key(stream_params);
    let cluster_coordinator = db::get_coordinator().await;

    match event {
        PipelineTableEvent::Remove => {
            #[cfg(feature = "enterprise")]
            if o2_enterprise::enterprise::common::infra::config::get_config()
                .super_cluster
                .enabled
            {
                if let Err(e) = o2_enterprise::enterprise::super_cluster::queue::delete(
                    &cache_key, false, true, None,
                )
                .await
                {
                    log::error!("[Pipeline] error triggering super cluster event to remove pipeline from cache: {e}");
                }
            }

            if let Err(e) = cluster_coordinator
                .delete(&cache_key, false, true, None)
                .await
            {
                log::error!("[Pipeline] error triggering event to remove pipeline from cache: {e}");
            }
        }
        PipelineTableEvent::Add => {
            if let Err(e) = cluster_coordinator
                .put(&cache_key, Bytes::new(), true, None)
                .await
            {
                log::error!("[Pipeline] error triggering event to add pipeline to cache: {e}");
            }

            // super cluster
            #[cfg(feature = "enterprise")]
            if o2_enterprise::enterprise::common::infra::config::get_config()
                .super_cluster
                .enabled
            {
                if let Err(e) = o2_enterprise::enterprise::super_cluster::queue::put(
                    &cache_key,
                    Bytes::new(),
                    true,
                    None,
                )
                .await
                {
                    log::error!("[Pipeline] error triggering super cluster event to add pipeline to cache: {e}");
                }
            }
        }
    }
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(PIPELINES_WATCH_PREFIX).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("[Pipeline::watch] His watch is started");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_pipelines: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let stream_params_str = ev.key.strip_prefix(PIPELINES_WATCH_PREFIX).unwrap();
                let Ok(stream_params) = stream_params_str.parse::<StreamParams>() else {
                    log::error!("[Pipeline::watch] error StreamParams from watch event key: {stream_params_str}");
                    continue;
                };
                let Some(pipeline) = get_by_stream(&stream_params).await else {
                    log::error!("[Pipeline::watch] error getting pipeline by id from db");
                    continue;
                };
                // Only realtime & enabled pipeline would trigger watch event -> compile directly
                match ExecutablePipeline::new(&pipeline).await {
                    Err(e) => {
                        log::error!(
                            "[Pipeline::watch] {}/{}/{}: Error initializing pipeline into ExecutablePipeline when updating cache: {}",
                            pipeline.org,
                            pipeline.name,
                            pipeline.id,
                            e
                        );
                    }
                    Ok(exec_pl) => {
                        let mut stream_pl_exec = STREAM_EXECUTABLE_PIPELINES.write().await;
                        stream_pl_exec.insert(stream_params, exec_pl);
                        log::info!(
                            "[Pipeline::watch]: pipeline {} added to cache.",
                            &pipeline.id
                        );
                    }
                };
            }
            db::Event::Delete(ev) => {
                let stream_params_str = ev.key.strip_prefix(PIPELINES_WATCH_PREFIX).unwrap();
                let Ok(stream_params) = stream_params_str.parse::<StreamParams>() else {
                    log::error!("[Pipeline::watch] error StreamParams from watch event key: {stream_params_str}");
                    continue;
                };
                if let Some(removed) = STREAM_EXECUTABLE_PIPELINES
                    .write()
                    .await
                    .remove(&stream_params)
                {
                    log::info!(
                        "[Pipeline]: pipeline {} removed from cache.",
                        removed.get_pipeline_id()
                    );
                };
            }
            db::Event::Empty => {}
        }
    }

    log::info!("[Pipeline::watch] His watch is ended");
    Ok(())
}

#[derive(Debug)]
enum PipelineTableEvent {
    Add,
    Remove,
}

fn get_cache_key(stream_params: &StreamParams) -> String {
    let stream_params_str = stream_params.to_string();
    format!("{}/{}", PIPELINES_WATCH_PREFIX, stream_params_str)
}
