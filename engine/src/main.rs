mod config;
mod dependency;
mod downloader;
mod logger;
mod protocol;

use config::EngineConfig;
use dependency::DependencyManager;
use downloader::DownloadManager;
use logger::Logger;
use protocol::{EngineRequest, EngineResponse, NativeMessageIo, RequestAction, ResponseType};
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tokio::sync::mpsc;

const ENGINE_VERSION: &str = "1.0.0";
const ENGINE_NAME: &str = "SnapLoad Windows Desktop Engine";

#[tokio::main]
async fn main() {
    Logger::info(&format!("Starting {} v{}", ENGINE_NAME, ENGINE_VERSION));

    let config = Arc::new(EngineConfig::load());
    let download_manager = DownloadManager::new();

    // Channel for outbound Native Messaging packets to stdout
    let (tx_outbound, mut rx_outbound) = mpsc::channel::<EngineResponse>(128);

    // Dedicated writer task ensuring serialized, atomic writes to stdout
    let writer_task = tokio::spawn(async move {
        let mut out = stdout();
        while let Some(response) = rx_outbound.recv().await {
            match serde_json::to_string(&response) {
                Ok(json_str) => {
                    if let Err(e) = NativeMessageIo::write_message(&mut out, &json_str).await {
                        Logger::error(&format!("Failed writing native message to stdout: {}", e));
                        break;
                    }
                }
                Err(e) => {
                    Logger::error(&format!("Failed serializing EngineResponse: {}", e));
                }
            }
        }
    });

    let mut in_reader = stdin();

    loop {
        match NativeMessageIo::read_message(&mut in_reader).await {
            Ok(Some(raw_msg)) => {
                if raw_msg.trim().is_empty() {
                    continue;
                }
                Logger::debug(&format!("Received message: {}", raw_msg));

                let request: Result<EngineRequest, _> = serde_json::from_str(&raw_msg);
                match request {
                    Ok(req) => {
                        let tx = tx_outbound.clone();
                        let cfg = (*config).clone();
                        let dl_mgr = download_manager.clone();

                        tokio::spawn(async move {
                            handle_request(req, cfg, dl_mgr, tx).await;
                        });
                    }
                    Err(e) => {
                        Logger::warn(&format!("Malformed JSON received: {}", e));
                        let _ = tx_outbound.send(EngineResponse::err(
                            "null",
                            "MALFORMED_JSON",
                            &format!("Failed to parse JSON message: {}", e),
                        )).await;
                    }
                }
            }
            Ok(None) => {
                // EOF on stdin: browser extension or host has disconnected
                Logger::info("Browser closed Native Messaging connection (EOF on stdin). Exiting cleanly.");
                break;
            }
            Err(e) => {
                Logger::error(&format!("Error reading native message from stdin: {}", e));
                break;
            }
        }
    }

    // Drop sender so writer task exits cleanly
    drop(tx_outbound);
    let _ = writer_task.await;
    Logger::info("SnapLoad Engine terminated cleanly.");
}

async fn handle_request(
    req: EngineRequest,
    config: EngineConfig,
    downloader: DownloadManager,
    tx: mpsc::Sender<EngineResponse>,
) {
    let req_id = req.id;
    match req.action {
        RequestAction::Ping => {
            let _ = tx.send(EngineResponse::ok(
                &req_id,
                ResponseType::Pong,
                serde_json::json!({ "timestamp": Logger::timestamp_utc() }),
            )).await;
        }
        RequestAction::GetVersion => {
            let _ = tx.send(EngineResponse::ok(
                &req_id,
                ResponseType::VersionInfo,
                serde_json::json!({
                    "name": ENGINE_NAME,
                    "version": ENGINE_VERSION,
                    "author": "DarkSkyee",
                    "platform": "windows-x86_64",
                }),
            )).await;
        }
        RequestAction::CheckDependencies => {
            let status = DependencyManager::check_status(&config).await;
            let _ = tx.send(EngineResponse::ok(
                &req_id,
                ResponseType::DependencyStatus,
                serde_json::to_value(status).unwrap_or_default(),
            )).await;
        }
        RequestAction::GetConfig => {
            let _ = tx.send(EngineResponse::ok(
                &req_id,
                ResponseType::Config,
                serde_json::to_value(&config).unwrap_or_default(),
            )).await;
        }
        RequestAction::StartDownload(payload) => {
            downloader.start_download(req_id, payload, config, tx).await;
        }
        RequestAction::CancelDownload(payload) => {
            let cancelled = downloader.cancel_download(&payload.download_id).await;
            let _ = tx.send(EngineResponse::ok(
                &req_id,
                ResponseType::DownloadCancelled,
                serde_json::json!({
                    "download_id": payload.download_id,
                    "success": cancelled,
                }),
            )).await;
        }
    }
}
