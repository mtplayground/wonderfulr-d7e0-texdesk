use serde::Serialize;

use crate::config::AppConfig;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

pub type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub fn get_app_config() -> CommandResult<AppConfig> {
    Ok(AppConfig::from_env())
}

#[tauri::command]
pub fn ping() -> CommandResult<String> {
    Ok("ok".to_owned())
}
