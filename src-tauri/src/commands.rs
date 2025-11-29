use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::{db::AppSettings, error::AppError, state::SharedState};

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
