//! Silicium desktop shell. It hosts the web UI (apps/web), holds the node identity and runs the
//! worker node as a child process.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod control_client;
mod deep_link;
mod journal;
mod link;
mod link_record;
mod node_config;
mod node_process;
mod signer;

use link_record::{LinkRecord, LinkRecordStore};
use node_process::NodeProcess;
use serde::Serialize;
use serde_json::Value;
use signer::{KeyStore, LinkProof};
use tauri::{Manager, State};
use tauri_plugin_deep_link::DeepLinkExt;

/// Stable error code for the UI (see the `code()` methods).
type CommandError = &'static str;

#[derive(Serialize)]
struct NodeStatus {
    running: bool,
    identity: Option<String>,
}

#[tauri::command]
fn machine_key(store: State<'_, KeyStore>) -> Result<Option<String>, CommandError> {
    store.public_key().map_err(|e| e.code())
}

#[tauri::command]
fn create_machine_key(store: State<'_, KeyStore>) -> Result<String, CommandError> {
    store.create().map_err(|e| e.code())
}

#[tauri::command]
fn sign_link_code(store: State<'_, KeyStore>, code: String) -> Result<LinkProof, CommandError> {
    store.link_proof(&code).map_err(|e| e.code())
}

/// The saved link, when it belongs to the machine key this app holds now.
#[tauri::command]
fn link_record(
    store: State<'_, KeyStore>,
    links: State<'_, LinkRecordStore>,
) -> Result<Option<LinkRecord>, CommandError> {
    let Some(record) = links.read() else {
        return Ok(None);
    };
    let current = store.public_key().map_err(|e| e.code())?;
    Ok((current.as_deref() == Some(record.public_key.as_str())).then_some(record))
}

/// Saves the link of this machine. Refused when the record is not for the current machine key.
#[tauri::command]
fn save_link_record(
    store: State<'_, KeyStore>,
    links: State<'_, LinkRecordStore>,
    record: LinkRecord,
) -> Result<(), CommandError> {
    let current = store.public_key().map_err(|e| e.code())?;
    if current.as_deref() != Some(record.public_key.as_str()) {
        return Err("no_key");
    }
    links.write(&record)
}

#[tauri::command]
fn node_status(process: State<'_, NodeProcess>, store: State<'_, KeyStore>) -> NodeStatus {
    NodeStatus {
        running: process.is_running(),
        identity: store.public_key().ok().flatten(),
    }
}

#[tauri::command]
fn node_start(process: State<'_, NodeProcess>) -> Result<(), CommandError> {
    process.start().map_err(|e| e.code())
}

#[tauri::command]
fn node_stop(process: State<'_, NodeProcess>) {
    process.stop();
}

#[tauri::command]
fn worker_report(process: State<'_, NodeProcess>) -> Result<Value, CommandError> {
    process.worker_report().map_err(|e| e.code())
}

fn main() {
    let result = tauri::Builder::default()
        // A second launch (from a link) hands its arguments to the running window and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for arg in argv {
                deep_link::offer(app, &arg);
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            // The node keeps its identity in its own folder; the link proof uses that identity.
            let node_dir = dir.join("node");
            app.manage(KeyStore::new(node_dir.join("node.seed")));
            app.manage(LinkRecordStore::new(node_dir.join("link.json")));
            let node = NodeProcess::new(node_dir);
            // An activated node that is not running (after a reboot or a crash) starts again.
            if node.wants_to_run() && !node.is_running() {
                let _ = node.start();
            }
            app.manage(node);
            app.manage(deep_link::PendingLink::default());
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    deep_link::offer(&handle, url.as_str());
                }
            });
            // A link that launched the application is delivered at start-up.
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    deep_link::offer(app.handle(), url.as_str());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            machine_key,
            create_machine_key,
            sign_link_code,
            link_record,
            save_link_record,
            node_status,
            node_start,
            node_stop,
            worker_report,
            deep_link::take_pending_link
        ])
        .build(tauri::generate_context!());
    match result {
        // Closing the window does not stop an activated node: it runs until "Désactiver ce PC".
        Ok(app) => app.run(|_handle, _event| {}),
        Err(error) => {
            eprintln!("silicium-desktop: {error}");
            std::process::exit(1);
        }
    }
}
