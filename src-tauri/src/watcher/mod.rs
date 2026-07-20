use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub const WORKSPACE_CHANGED_EVENT: &str = "workspace-changed";

#[derive(Default)]
pub struct WorkspaceWatcherState {
    active: Mutex<Option<ActiveWatcher>>,
}

struct ActiveWatcher {
    workspace_root: PathBuf,
    _watcher: RecommendedWatcher,
}

#[derive(Debug)]
pub enum WatcherError {
    InvalidWorkspaceRoot,
    StateLock,
    Notify {
        action: &'static str,
        source: notify::Error,
    },
}

impl WatcherError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidWorkspaceRoot => "watcher_invalid_workspace_root",
            Self::StateLock => "watcher_state_lock",
            Self::Notify { .. } => "watcher_notify",
        }
    }
}

impl fmt::Display for WatcherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspaceRoot => write!(formatter, "workspace root must be a directory"),
            Self::StateLock => write!(formatter, "workspace watcher state is unavailable"),
            Self::Notify { action, source } => write!(formatter, "{action} failed: {source}"),
        }
    }
}

impl std::error::Error for WatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Notify { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchWorkspaceRequest {
    pub workspace_root: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceWatchStatus {
    pub active: bool,
    pub workspace_root: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceChangeEvent {
    pub workspace_root: String,
    pub paths: Vec<String>,
    pub kind: String,
}

impl WorkspaceWatcherState {
    pub fn start(
        &self,
        app_handle: AppHandle,
        request: WatchWorkspaceRequest,
    ) -> Result<WorkspaceWatchStatus, WatcherError> {
        let workspace_root = canonical_workspace_root(&request.workspace_root)?;
        let event_root = workspace_root.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let payload = event_payload(&event_root, event);
                    if !payload.paths.is_empty() {
                        let _ = app_handle.emit(WORKSPACE_CHANGED_EVENT, payload);
                    }
                }
            })
            .map_err(|source| WatcherError::Notify {
                action: "create watcher",
                source,
            })?;

        watcher
            .watch(&workspace_root, RecursiveMode::Recursive)
            .map_err(|source| WatcherError::Notify {
                action: "watch workspace",
                source,
            })?;

        let mut active = self.active.lock().map_err(|_| WatcherError::StateLock)?;
        *active = Some(ActiveWatcher {
            workspace_root: workspace_root.clone(),
            _watcher: watcher,
        });

        Ok(WorkspaceWatchStatus {
            active: true,
            workspace_root: Some(path_to_string(&workspace_root)),
        })
    }

    pub fn stop(&self) -> Result<WorkspaceWatchStatus, WatcherError> {
        let mut active = self.active.lock().map_err(|_| WatcherError::StateLock)?;
        *active = None;
        Ok(WorkspaceWatchStatus {
            active: false,
            workspace_root: None,
        })
    }

    pub fn status(&self) -> Result<WorkspaceWatchStatus, WatcherError> {
        let active = self.active.lock().map_err(|_| WatcherError::StateLock)?;
        Ok(WorkspaceWatchStatus {
            active: active.is_some(),
            workspace_root: active
                .as_ref()
                .map(|watcher| path_to_string(&watcher.workspace_root)),
        })
    }
}

fn canonical_workspace_root(root: &str) -> Result<PathBuf, WatcherError> {
    let canonical_root = Path::new(root)
        .canonicalize()
        .map_err(|_| WatcherError::InvalidWorkspaceRoot)?;
    let metadata = std::fs::metadata(&canonical_root).map_err(|_| WatcherError::InvalidWorkspaceRoot)?;
    if metadata.is_dir() {
        Ok(canonical_root)
    } else {
        Err(WatcherError::InvalidWorkspaceRoot)
    }
}

fn event_payload(root: &Path, event: Event) -> WorkspaceChangeEvent {
    let paths = event
        .paths
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(path_to_string)
        .collect();

    WorkspaceChangeEvent {
        workspace_root: path_to_string(root),
        paths,
        kind: format!("{:?}", event.kind),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
