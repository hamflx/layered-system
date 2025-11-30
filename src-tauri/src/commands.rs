use std::path::PathBuf;

use serde::Serialize;
use tauri::async_runtime::spawn_blocking;
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

async fn run_blocking_cmd<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> CmdResult<T> + Send + 'static,
{
    spawn_blocking(f)
        .await
        .map_err(|e| format!("failed to join async task: {e}"))?
}

#[tauri::command]
pub async fn check_admin() -> CmdResult<bool> {
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
pub async fn init_root(
    root_path: String,
    locale: Option<String>,
    state: State<'_, SharedState>,
) -> CmdResult<InitResult> {
    let root_path = PathBuf::from(root_path);
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let settings = state
            .initialize(root_path, locale)
            .map_err(|e| e.to_string())?;
        Ok(InitResult { settings })
    })
    .await
}

#[tauri::command]
pub async fn get_settings(state: State<'_, SharedState>) -> CmdResult<Option<AppSettings>> {
    let state = state.inner().clone();
    run_blocking_cmd(move || match state.get_settings() {
        Ok(settings) => Ok(settings),
        Err(AppError::RootNotInitialized) => Ok(None),
        Err(other) => Err(other.to_string()),
    })
    .await
}

#[tauri::command]
pub async fn scan_workspace(state: State<'_, SharedState>) -> CmdResult<Vec<Node>> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.scan().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn list_nodes(state: State<'_, SharedState>) -> CmdResult<Vec<Node>> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.list_nodes().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn list_wim_images(
    image_path: String,
    state: State<'_, SharedState>,
) -> CmdResult<Vec<WimImageInfo>> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.list_wim_images(&image_path).map_err(|e| e.to_string())
    })
    .await
}

#[derive(Serialize)]
pub struct CreateNodeResponse {
    pub node: Node,
}

#[tauri::command]
pub async fn create_base_vhd(
    name: String,
    desc: Option<String>,
    wim_file: String,
    wim_index: u32,
    size_gb: u64,
    state: State<'_, SharedState>,
) -> CmdResult<CreateNodeResponse> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        let node = svc
            .create_base(&name, desc, &wim_file, wim_index, size_gb)
            .map_err(|e| e.to_string())?;
        Ok(CreateNodeResponse { node })
    })
    .await
}

#[tauri::command]
pub async fn create_diff_vhd(
    parent_id: String,
    name: String,
    desc: Option<String>,
    state: State<'_, SharedState>,
) -> CmdResult<CreateNodeResponse> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        let node = svc
            .create_diff(&parent_id, &name, desc)
            .map_err(|e| e.to_string())?;
        Ok(CreateNodeResponse { node })
    })
    .await
}

#[tauri::command]
pub async fn set_bootsequence_and_reboot(
    node_id: String,
    state: State<'_, SharedState>,
) -> CmdResult<()> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.set_bootsequence_and_reboot(&node_id)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_subtree(node_id: String, state: State<'_, SharedState>) -> CmdResult<()> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.delete_subtree(&node_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_bcd(node_id: String, state: State<'_, SharedState>) -> CmdResult<()> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.delete_bcd(&node_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn repair_bcd(
    node_id: String,
    state: State<'_, SharedState>,
) -> CmdResult<Option<String>> {
    let state = state.inner().clone();
    run_blocking_cmd(move || {
        let svc = WorkspaceService::new(state);
        svc.repair_bcd(&node_id).map_err(|e| e.to_string())
    })
    .await
}
