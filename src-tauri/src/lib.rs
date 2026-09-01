// SPDX-License-Identifier: GPL-3.0-or-later
mod commands;
mod dto;
mod mime;
mod protocol;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .register_uri_scheme_protocol(protocol::SCHEME, protocol::handle_request)
        .invoke_handler(tauri::generate_handler![
            commands::open_archive,
            commands::query_messages,
            commands::get_day_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
