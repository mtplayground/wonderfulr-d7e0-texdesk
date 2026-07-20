use serde::{Deserialize, Serialize};
use tauri::State;

use crate::compile::{CompileDocumentRequest, CompileError, CompileResult};
use crate::config::AppConfig;
use crate::fs::{
    CreateFileRequest, DeleteResult, FileContent, FsEntry, FsError, ListWorkspaceRequest,
    RenameEntryRequest, WorkspacePathRequest, WriteFileRequest,
};
use crate::store::{
    RecentProject, Store, StoreError, StoreStatus, Template, TemplateInput, WorkspaceState,
};
use crate::watcher::{
    WatchWorkspaceRequest, WatcherError, WorkspaceWatchStatus, WorkspaceWatcherState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RememberWorkspaceRequest {
    pub workspace_root: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RememberOpenFileRequest {
    pub workspace_root: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectsRequest {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTemplateRequest {
    pub workspace_root: String,
    pub target_directory: String,
    pub template_id: String,
    pub assignment_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTemplate {
    pub main_file: FsEntry,
    pub bibliography_file: Option<FsEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTemplateRequest {
    pub template: TemplateInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTemplateRequest {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTemplateResult {
    pub deleted_id: String,
}

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

impl From<WatcherError> for CommandError {
    fn from(error: WatcherError) -> Self {
        Self {
            code: error.code().to_owned(),
            message: error.to_string(),
        }
    }
}

impl From<CompileError> for CommandError {
    fn from(error: CompileError) -> Self {
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
pub fn get_workspace_state(store: State<'_, Store>) -> CommandResult<WorkspaceState> {
    store.workspace_state().map_err(CommandError::from)
}

#[tauri::command]
pub fn remember_workspace_root(
    request: RememberWorkspaceRequest,
    store: State<'_, Store>,
) -> CommandResult<WorkspaceState> {
    store
        .remember_workspace_root(&request.workspace_root)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn remember_open_file(
    request: RememberOpenFileRequest,
    store: State<'_, Store>,
) -> CommandResult<WorkspaceState> {
    store
        .remember_open_file(&request.workspace_root, &request.path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_recent_projects(
    request: RecentProjectsRequest,
    store: State<'_, Store>,
) -> CommandResult<Vec<RecentProject>> {
    store
        .recent_projects(request.limit.unwrap_or(10))
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_templates(store: State<'_, Store>) -> CommandResult<Vec<Template>> {
    store.templates().map_err(CommandError::from)
}

#[tauri::command]
pub fn save_template(
    request: SaveTemplateRequest,
    store: State<'_, Store>,
) -> CommandResult<Template> {
    store
        .save_template(request.template)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_template(
    request: DeleteTemplateRequest,
    store: State<'_, Store>,
) -> CommandResult<DeleteTemplateResult> {
    store
        .delete_template(&request.id)
        .map(|deleted_id| DeleteTemplateResult { deleted_id })
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn apply_template_to_workspace(
    request: ApplyTemplateRequest,
    store: State<'_, Store>,
) -> CommandResult<AppliedTemplate> {
    let template = store
        .template(&request.template_id)
        .map_err(CommandError::from)?;
    let main_file_name = normalize_tex_file_name(&request.assignment_name)?;
    let main_file_path = join_workspace_path(&request.target_directory, &main_file_name);
    let bibliography_path = template
        .bibliography
        .as_ref()
        .map(|_| join_workspace_path(&request.target_directory, "references.bib"));

    crate::fs::ensure_path_available(&request.workspace_root, &main_file_path)
        .map_err(CommandError::from)?;
    if let Some(path) = &bibliography_path {
        crate::fs::ensure_path_available(&request.workspace_root, path)
            .map_err(CommandError::from)?;
    }

    let main_file = crate::fs::create_file(CreateFileRequest {
        workspace_root: request.workspace_root.clone(),
        path: main_file_path,
        contents: Some(template.body),
    })
    .map_err(CommandError::from)?;

    let bibliography_file = match (bibliography_path, template.bibliography) {
        (Some(path), Some(contents)) => Some(
            crate::fs::create_file(CreateFileRequest {
                workspace_root: request.workspace_root,
                path,
                contents: Some(contents),
            })
            .map_err(CommandError::from)?,
        ),
        _ => None,
    };

    Ok(AppliedTemplate {
        main_file,
        bibliography_file,
    })
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

#[tauri::command]
pub fn compile_document(request: CompileDocumentRequest) -> CommandResult<CompileResult> {
    crate::compile::compile_document(request).map_err(CommandError::from)
}

#[tauri::command]
pub fn start_workspace_watcher(
    app_handle: tauri::AppHandle,
    request: WatchWorkspaceRequest,
    watcher: State<'_, WorkspaceWatcherState>,
) -> CommandResult<WorkspaceWatchStatus> {
    watcher
        .start(app_handle, request)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn stop_workspace_watcher(
    watcher: State<'_, WorkspaceWatcherState>,
) -> CommandResult<WorkspaceWatchStatus> {
    watcher.stop().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_workspace_watcher_status(
    watcher: State<'_, WorkspaceWatcherState>,
) -> CommandResult<WorkspaceWatchStatus> {
    watcher.status().map_err(CommandError::from)
}

fn normalize_tex_file_name(raw_name: &str) -> CommandResult<String> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err(CommandError {
            code: "template_invalid_assignment_name".to_owned(),
            message: "assignment file name is required".to_owned(),
        });
    }

    let file_name = if trimmed.to_lowercase().ends_with(".tex") {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.tex")
    };

    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".." {
        return Err(CommandError {
            code: "template_invalid_assignment_name".to_owned(),
            message: "assignment file name cannot contain path separators".to_owned(),
        });
    }

    Ok(file_name)
}

fn join_workspace_path(directory: &str, file_name: &str) -> String {
    let clean_directory = directory.trim_matches('/');
    if clean_directory.is_empty() {
        file_name.to_owned()
    } else {
        format!("{clean_directory}/{file_name}")
    }
}
