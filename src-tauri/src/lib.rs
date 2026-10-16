// SPDX-License-Identifier: GPL-3.0-or-later
mod chrome;
mod commands;
mod dto;
mod export;
mod heic;
mod ingest;
mod media_size;
mod mime;
mod protocol;
mod rendition_cache;
mod sidecar;
mod tools;

use chrome::ChromeState;
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
        .manage(ChromeState::default())
        .setup(|app| {
            chrome::setup(app)?;
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle_request)
        .invoke_handler(tauri::generate_handler![
            chrome::chrome_info,
            commands::open_archive,
            commands::query_messages,
            commands::get_day_index,
            export::save_attachment,
            export::export_conversation,
            ingest::check_ingest_prerequisites,
            ingest::list_ingest_devices,
            ingest::device_backup_cache_info,
            ingest::clear_device_backup_cache,
            ingest::begin_ingest,
            ingest::ingest_conversation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
