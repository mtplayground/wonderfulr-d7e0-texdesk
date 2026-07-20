use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::fs::{
    CreateFileRequest, DeleteResult, FileContent, FsEntry, FsError, ListWorkspaceRequest,
    RenameEntryRequest, WorkspacePathRequest, WriteFileRequest,
};
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

impl From<FsError> for CommandError {
    fn from(error: FsError) -> Self {
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

#[tauri::command]
pub fn list_workspace_entries(request: ListWorkspaceRequest) -> CommandResult<Vec<FsEntry>> {
    crate::fs::list_entries(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn read_workspace_file(request: WorkspacePathRequest) -> CommandResult<FileContent> {
    crate::fs::read_file(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn write_workspace_file(request: WriteFileRequest) -> CommandResult<FsEntry> {
    crate::fs::write_file(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn create_workspace_file(request: CreateFileRequest) -> CommandResult<FsEntry> {
    crate::fs::create_file(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn create_workspace_directory(request: WorkspacePathRequest) -> CommandResult<FsEntry> {
    crate::fs::create_directory(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn rename_workspace_entry(request: RenameEntryRequest) -> CommandResult<FsEntry> {
    crate::fs::rename_entry(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_workspace_entry(request: WorkspacePathRequest) -> CommandResult<DeleteResult> {
    crate::fs::delete_entry(request).map_err(CommandError::from)
}
