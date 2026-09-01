// SPDX-License-Identifier: GPL-3.0-or-later
mod commands;
mod dto;
mod ingest;
mod mime;
mod protocol;
mod sidecar;
mod tools;

use commands::AppState;
use ingest::IngestState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .manage(IngestState::default())
        .register_uri_scheme_protocol(protocol::SCHEME, protocol::handle_request)
        .invoke_handler(tauri::generate_handler![
            commands::open_archive,
            commands::query_messages,
            commands::get_day_index,
            ingest::check_ingest_prerequisites,
            ingest::list_ingest_devices,
            ingest::install_usb_driver,
            ingest::begin_ingest,
            ingest::ingest_conversation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
