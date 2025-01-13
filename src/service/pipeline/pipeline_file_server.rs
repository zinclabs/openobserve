// Copyright 2025 OpenObserve Inc.
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

use std::path;

use infra::errors::Result;
use ingester::{Entry, WAL_DIR_DEFAULT_PREFIX};
use tokio::sync::{mpsc, oneshot};
use wal::Writer;

use crate::service::pipeline::{
    pipeline_entry::PipelineEntry,
    pipeline_exporter::PipelineExporter,
    pipeline_watcher::{PipelineWatcher, WatcherEvent, FILE_WATCHER_NOTIFY},
};

pub struct PipelineFileServerBuilder {}

impl PipelineFileServerBuilder {
    pub async fn build() -> PipelineFileServer {
        let cfg = &config::get_config();

        // build pipeline_watcher
        let (s, r) = mpsc::channel(cfg.pipeline.remote_stream_wal_concurrent_count);
        let pipeline_watcher =
            PipelineWatcher::init(s, r, cfg.pipeline.remote_stream_wal_concurrent_count).await;

        // build pipeline_exporter
        let (stop_pipeline_expoter_tx, stop_pipleline_exporter_rx) = mpsc::channel(1);
        let (stop_pipeline_watcher_tx, stop_pipleline_watcher_rx) = mpsc::channel(1);
        let (pipeline_exporter_tx, pipeline_exporter_rx) = mpsc::channel::<PipelineEntry>(1);
        let pipelins_exporter =
            PipelineExporter::init(pipeline_exporter_rx, stop_pipleline_exporter_rx).unwrap();

        PipelineFileServer::new(
            pipeline_watcher,
            pipelins_exporter,
            pipeline_exporter_tx,
            stop_pipeline_expoter_tx,
            stop_pipeline_watcher_tx,
            stop_pipleline_watcher_rx,
        )
    }
}

pub struct PipelineFileServer {
    pub pipeline_watcher: PipelineWatcher,
    pub pipeline_exporter: PipelineExporter,
    pipeline_exporter_sender: mpsc::Sender<PipelineEntry>,
    stop_pipeline_expoter_tx: mpsc::Sender<()>,
    stop_pipeline_watcher_tx: mpsc::Sender<()>,
    stop_pipeline_watcher_rx: mpsc::Receiver<()>,
}

impl PipelineFileServer {
    pub fn new(
        pipeline_watcher: PipelineWatcher,
        pipeline_exporter: PipelineExporter,
        pipeline_exporter_sender: mpsc::Sender<PipelineEntry>,
        stop_pipeline_expoter_tx: mpsc::Sender<()>,
        stop_pipeline_watcher_tx: mpsc::Sender<()>,
        stop_pipeline_watcher_rx: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            pipeline_watcher,
            pipeline_exporter,
            stop_pipeline_expoter_tx,
            pipeline_exporter_sender,
            stop_pipeline_watcher_tx,
            stop_pipeline_watcher_rx,
        }
    }

    pub fn destructure(
        self,
    ) -> (
        PipelineWatcher,
        PipelineExporter,
        mpsc::Sender<PipelineEntry>,
        mpsc::Sender<()>,
        mpsc::Sender<()>,
        mpsc::Receiver<()>,
    ) {
        (
            self.pipeline_watcher,
            self.pipeline_exporter,
            self.pipeline_exporter_sender,
            self.stop_pipeline_expoter_tx,
            self.stop_pipeline_watcher_tx,
            self.stop_pipeline_watcher_rx,
        )
    }

    pub async fn run(outside_shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
        log::info!("PipelineFileServer started");
        let (
            mut pipeline_watcher,
            mut pipeline_exporter,
            pipeline_exporter_sender,
            stop_pipeline_exporter_sender,
            stop_pipeline_watcher_sender,
            stop_pipeline_watcher_rx,
        ) = PipelineFileServerBuilder::build().await.destructure();

        tokio::spawn(async move {
            log::info!("PipelineWatcher started");
            let _ = pipeline_watcher
                .run(pipeline_exporter_sender, stop_pipeline_watcher_rx)
                .await;
            log::info!("PipelineWatcher end");
        });

        tokio::spawn(async move {
            log::info!("PipelineExporter started");
            let _ = pipeline_exporter.export().await;
            log::info!("PipelineExporter end");
        });

        tokio::spawn(async move {
            log::info!("PipelineFileServer running and waiting for shutdown signal");
            loop {
                let _ = outside_shutdown_rx.await;
                println!("PipelineExporter begin to shutdown");
                log::info!("PipelineExporter begin to shutdown");
                let _ = stop_pipeline_exporter_sender.send(()).await;

                log::info!("PipelineWatcher begin to shutdown");
                let _ = stop_pipeline_watcher_sender.send(()).await;

                break;
            }
            log::info!("PipelineFileServer end");
        });

        // debug
        tokio::spawn(async {
            PipelineFileServer::debug_wal_send().await;
            // loop {
            //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            //     PipelineFileServer::debug_wal_send().await;
            // }
        });

        Ok(())
    }

    async fn debug_wal_send() {
        use std::sync::Arc;

        use parquet::data_type::AsBytes;
        let config = &config::get_config();
        let dir = path::PathBuf::from(&config.pipeline.remote_stream_wal_dir)
            .join(WAL_DIR_DEFAULT_PREFIX);
        let mut writer = Writer::new(dir, "org", "stream", 1, 1024_1024, 8 * 1024).unwrap();

        for i in 0..100 {
            let data = serde_json::json!(
              {
                "Athlete": "Alfred",
                "City": "Athens",
                "Country": "HUN",
                "Discipline": "Swimming",
                "Sport": "Aquatics",
                "Year": 1896,
                "order" : i,
              }
            );

            let mut entry = Entry {
                stream: Arc::from("example_stream"),
                schema: None, // empty Schema
                schema_key: Arc::from("example_schema_key"),
                partition_key: Arc::from("2023/12/18/00/country=US/state=CA"),
                data: vec![Arc::new(data.clone())],
                data_size: 1,
            };
            let bytes_entries = entry.into_bytes().unwrap();
            writer.write(bytes_entries.as_bytes()).unwrap();
        }
        writer.close().unwrap();

        // notify file watcher to load a new wal file
        log::info!(
            "notify file watcher to load a new wal file: {}",
            writer.path().display()
        );
        let watcher = FILE_WATCHER_NOTIFY.write().await;
        let _ = watcher
            .clone()
            .unwrap()
            .send(WatcherEvent::NewFile(writer.path().clone()))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use crate::service::pipeline::pipeline_file_server::PipelineFileServer;

    #[tokio::test]
    async fn test_pipeline_file_server() {
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(PipelineFileServer::run(stop_rx));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        stop_tx.send(()).unwrap();
    }
}
