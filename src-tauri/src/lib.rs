mod commands;
mod db;
mod error;
mod logging;
mod paths;
mod state;
mod sys;
mod temp;

use state::SharedState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state = SharedState::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            commands::check_admin,
            commands::get_settings,
            commands::init_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
