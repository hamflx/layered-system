use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::{
    db::AppSettings,
    error::AppError,
    models::{Node, WimImageInfo},
    state::SharedState,
    workspace::WorkspaceService,
};

type CmdResult<T> = std::result::Result<T, String>;

#[derive(Serialize)]
pub struct InitResult {
    pub settings: AppSettings,
}

#[tauri::command]
pub fn check_admin() -> CmdResult<bool> {
    #[cfg(windows)]
    {
        Ok(is_elevated::is_elevated())
    }
    #[cfg(not(windows))]
    {
        Ok(true)
    }
}

#[tauri::command]
pub fn init_root(
    root_path: String,
    locale: Option<String>,
    state: State<SharedState>,
) -> CmdResult<InitResult> {
    let root_path = PathBuf::from(root_path);
    let settings = state
        .initialize(root_path, locale)
        .map_err(|e| e.to_string())?;
    Ok(InitResult { settings })
}

#[tauri::command]
pub fn get_settings(state: State<SharedState>) -> CmdResult<Option<AppSettings>> {
    match state.get_settings() {
        Ok(settings) => Ok(settings),
        Err(AppError::RootNotInitialized) => Ok(None),
        Err(other) => Err(other.to_string()),
    }
}

#[tauri::command]
pub fn scan_workspace(state: State<SharedState>) -> CmdResult<Vec<Node>> {
    let svc = WorkspaceService::new(&state);
    svc.scan().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_wim_images(
    image_path: String,
    state: State<SharedState>,
) -> CmdResult<Vec<WimImageInfo>> {
    let svc = WorkspaceService::new(&state);
    svc.list_wim_images(&image_path).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct CreateNodeResponse {
    pub node: Node,
}

#[tauri::command]
pub fn create_base_vhd(
    name: String,
    desc: Option<String>,
    wim_file: String,
    wim_index: u32,
    size_gb: u64,
    state: State<SharedState>,
) -> CmdResult<CreateNodeResponse> {
    let svc = WorkspaceService::new(&state);
    let node = svc
        .create_base(&name, desc, &wim_file, wim_index, size_gb)
        .map_err(|e| e.to_string())?;
    Ok(CreateNodeResponse { node })
}

#[tauri::command]
pub fn create_diff_vhd(
    parent_id: String,
    name: String,
    desc: Option<String>,
    state: State<SharedState>,
) -> CmdResult<CreateNodeResponse> {
    let svc = WorkspaceService::new(&state);
    let node = svc
        .create_diff(&parent_id, &name, desc)
        .map_err(|e| e.to_string())?;
    Ok(CreateNodeResponse { node })
}

#[tauri::command]
pub fn set_bootsequence_and_reboot(node_id: String, state: State<SharedState>) -> CmdResult<()> {
    let svc = WorkspaceService::new(&state);
    svc.set_bootsequence_and_reboot(&node_id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_subtree(node_id: String, state: State<SharedState>) -> CmdResult<()> {
    let svc = WorkspaceService::new(&state);
    svc.delete_subtree(&node_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn repair_bcd(node_id: String, state: State<SharedState>) -> CmdResult<Option<String>> {
    let svc = WorkspaceService::new(&state);
    svc.repair_bcd(&node_id).map_err(|e| e.to_string())
}
