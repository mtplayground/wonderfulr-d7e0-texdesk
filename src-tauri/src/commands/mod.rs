use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::store::{Store, StoreError, StoreStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

pub type CommandResult<T> = Result<T, CommandError>;

impl From<StoreError> for CommandError {
    fn from(error: StoreError) -> Self {
        Self {
            code: error.code().to_owned(),
            message: error.to_string(),
        }
    }
}

#[tauri::command]
pub fn get_app_config() -> CommandResult<AppConfig> {
    Ok(AppConfig::from_env())
}

#[tauri::command]
pub fn ping() -> CommandResult<String> {
    Ok("ok".to_owned())
}

#[tauri::command]
pub fn get_store_status(store: State<'_, Store>) -> CommandResult<StoreStatus> {
    store.status().map_err(CommandError::from)
}
