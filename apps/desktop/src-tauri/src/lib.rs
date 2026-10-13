pub mod commands;
pub mod monitoring;
pub mod server;

use commands::{get_process_list, get_system_snapshot, query_assistant};
use monitoring::{CollectorState, SystemCollector};
use server::start_pairing_server;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let collector = SystemCollector::new();
            let state = Arc::new(CollectorState {
                collector: Mutex::new(collector),
            });
            
            // Start local loopback pairing server for kernova.click integration
            start_pairing_server(state.clone());

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_snapshot,
            get_process_list,
            query_assistant
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
