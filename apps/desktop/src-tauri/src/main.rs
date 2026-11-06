//! Silicium desktop shell. It hosts the web UI (apps/web), holds the node identity and runs the
//! worker node as a child process.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod control_client;
mod node_process;
mod signer;

use node_process::NodeProcess;
use serde::Serialize;
use serde_json::Value;
use signer::{KeyStore, LinkProof};
use tauri::{Manager, State};

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
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            // The node keeps its identity in its own folder; the link proof uses that identity.
            let node_dir = dir.join("node");
            app.manage(KeyStore::new(node_dir.join("node.seed")));
            app.manage(NodeProcess::new(node_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            machine_key,
            create_machine_key,
            sign_link_code,
            node_status,
            node_start,
            node_stop,
            worker_report
        ])
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                handle.state::<NodeProcess>().stop();
            }
        }),
        Err(error) => {
            eprintln!("silicium-desktop: {error}");
            std::process::exit(1);
        }
    }
}
