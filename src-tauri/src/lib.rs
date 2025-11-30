mod bcd;
mod commands;
mod db;
mod diskpart;
mod dism;
mod error;
mod logging;
mod models;
mod paths;
mod state;
mod sys;
mod temp;
mod workspace;

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
            commands::init_root,
            commands::scan_workspace,
            commands::list_nodes,
            commands::list_wim_images,
            commands::create_base_vhd,
            commands::create_diff_vhd,
            commands::set_bootsequence_and_reboot,
            commands::delete_subtree,
            commands::delete_bcd,
            commands::repair_bcd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
