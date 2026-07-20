use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug)]
pub enum FsError {
    InvalidWorkspaceRoot,
    InvalidRelativePath { path: String },
    PathOutsideWorkspace { path: String },
    PathNotFound { path: String },
    DirectoryExpected { path: String },
    FileExpected { path: String },
    DestinationExists { path: String },
    Io { action: &'static str, source: std::io::Error },
}

impl FsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidWorkspaceRoot => "fs_invalid_workspace_root",
            Self::InvalidRelativePath { .. } => "fs_invalid_relative_path",
            Self::PathOutsideWorkspace { .. } => "fs_path_outside_workspace",
            Self::PathNotFound { .. } => "fs_path_not_found",
            Self::DirectoryExpected { .. } => "fs_directory_expected",
            Self::FileExpected { .. } => "fs_file_expected",
            Self::DestinationExists { .. } => "fs_destination_exists",
            Self::Io { .. } => "fs_io",
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspaceRoot => write!(formatter, "workspace root must be a directory"),
            Self::InvalidRelativePath { path } => {
                write!(formatter, "path must be relative to the workspace root: {path}")
            }
            Self::PathOutsideWorkspace { path } => {
                write!(formatter, "path is outside the workspace root: {path}")
            }
            Self::PathNotFound { path } => write!(formatter, "path was not found: {path}"),
            Self::DirectoryExpected { path } => write!(formatter, "directory expected: {path}"),
            Self::FileExpected { path } => write!(formatter, "file expected: {path}"),
            Self::DestinationExists { path } => {
                write!(formatter, "destination already exists: {path}")
            }
            Self::Io { action, source } => write!(formatter, "{action} failed: {source}"),
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePathRequest {
    pub workspace_root: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaceRequest {
    pub workspace_root: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteFileRequest {
    pub workspace_root: String,
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileRequest {
    pub workspace_root: String,
    pub path: String,
    pub contents: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameEntryRequest {
    pub workspace_root: String,
    pub from_path: String,
    pub to_path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub deleted_path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub kind: FsEntryKind,
    pub size_bytes: Option<u64>,
    pub modified_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FsEntryKind {
    Directory,
    File,
}

pub fn list_entries(request: ListWorkspaceRequest) -> Result<Vec<FsEntry>, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let relative_path = request.path.unwrap_or_default();
    let directory = resolve_existing_path(&root, &relative_path)?;
    let metadata = metadata_for(&directory, "read directory metadata")?;
    if !metadata.is_dir() {
        return Err(FsError::DirectoryExpected {
            path: relative_path,
        });
    }

    let mut entries = Vec::new();
    let reader = fs::read_dir(&directory).map_err(|source| FsError::Io {
        action: "read directory",
        source,
    })?;

    for entry_result in reader {
        let entry = entry_result.map_err(|source| FsError::Io {
            action: "read directory entry",
            source,
        })?;
        entries.push(entry_for_path(&root, &entry.path())?);
    }

    entries.sort_by(|left, right| {
        let kind_order = match (&left.kind, &right.kind) {
            (FsEntryKind::Directory, FsEntryKind::File) => std::cmp::Ordering::Less,
            (FsEntryKind::File, FsEntryKind::Directory) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };
        kind_order.then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(entries)
}

pub fn read_file(request: WorkspacePathRequest) -> Result<FileContent, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let path = resolve_existing_path(&root, &request.path)?;
    let metadata = metadata_for(&path, "read file metadata")?;
    if !metadata.is_file() {
        return Err(FsError::FileExpected { path: request.path });
    }

    let contents = fs::read_to_string(&path).map_err(|source| FsError::Io {
        action: "read file",
        source,
    })?;

    Ok(FileContent {
        path: relative_path_string(&root, &path)?,
        contents,
    })
}

pub fn write_file(request: WriteFileRequest) -> Result<FsEntry, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let mut path = resolve_writable_path(&root, &request.path)?;
    if path.exists() {
        path = path.canonicalize().map_err(|_| FsError::PathNotFound {
            path: request.path.clone(),
        })?;
        ensure_inside_workspace(&root, &path, &request.path)?;
        let metadata = metadata_for(&path, "read file metadata")?;
        if !metadata.is_file() {
            return Err(FsError::FileExpected { path: request.path });
        }
    }

    fs::write(&path, request.contents).map_err(|source| FsError::Io {
        action: "write file",
        source,
    })?;
    entry_for_path(&root, &path)
}

pub fn create_file(request: CreateFileRequest) -> Result<FsEntry, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let path = resolve_writable_path(&root, &request.path)?;
    let contents = request.contents.unwrap_or_default();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                FsError::DestinationExists {
                    path: request.path.clone(),
                }
            } else {
                FsError::Io {
                    action: "create file",
                    source,
                }
            }
        })?;
    std::io::Write::write_all(&mut file, contents.as_bytes()).map_err(|source| FsError::Io {
        action: "write new file",
        source,
    })?;
    entry_for_path(&root, &path)
}

pub fn create_directory(request: WorkspacePathRequest) -> Result<FsEntry, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let path = resolve_writable_path(&root, &request.path)?;
    if path.exists() {
        return Err(FsError::DestinationExists { path: request.path });
    }

    fs::create_dir(&path).map_err(|source| FsError::Io {
        action: "create directory",
        source,
    })?;
    entry_for_path(&root, &path)
}

pub fn rename_entry(request: RenameEntryRequest) -> Result<FsEntry, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let from_path = resolve_existing_path(&root, &request.from_path)?;
    let to_path = resolve_writable_path(&root, &request.to_path)?;
    if to_path.exists() {
        return Err(FsError::DestinationExists {
            path: request.to_path,
        });
    }

    fs::rename(&from_path, &to_path).map_err(|source| FsError::Io {
        action: "rename entry",
        source,
    })?;
    entry_for_path(&root, &to_path)
}

pub fn delete_entry(request: WorkspacePathRequest) -> Result<DeleteResult, FsError> {
    let root = canonical_workspace_root(&request.workspace_root)?;
    let path = resolve_existing_path(&root, &request.path)?;
    let metadata = metadata_for(&path, "read entry metadata")?;
    if metadata.is_dir() {
        fs::remove_dir_all(&path).map_err(|source| FsError::Io {
            action: "delete directory",
            source,
        })?;
    } else {
        fs::remove_file(&path).map_err(|source| FsError::Io {
            action: "delete file",
            source,
        })?;
    }

    Ok(DeleteResult {
        deleted_path: request.path,
    })
}

pub(crate) fn canonical_workspace_root(root: &str) -> Result<PathBuf, FsError> {
    let root_path = Path::new(root);
    let canonical_root = root_path
        .canonicalize()
        .map_err(|_| FsError::InvalidWorkspaceRoot)?;
    let metadata = metadata_for(&canonical_root, "read workspace metadata")?;
    if metadata.is_dir() {
        Ok(canonical_root)
    } else {
        Err(FsError::InvalidWorkspaceRoot)
    }
}

pub(crate) fn resolve_existing_path(root: &Path, relative_path: &str) -> Result<PathBuf, FsError> {
    let candidate = root.join(clean_relative_path(relative_path)?);
    let canonical_candidate = candidate.canonicalize().map_err(|_| FsError::PathNotFound {
        path: relative_path.to_owned(),
    })?;
    ensure_inside_workspace(root, &canonical_candidate, relative_path)?;
    Ok(canonical_candidate)
}

fn resolve_writable_path(root: &Path, relative_path: &str) -> Result<PathBuf, FsError> {
    let clean_path = clean_relative_path(relative_path)?;
    if clean_path.as_os_str().is_empty() {
        return Err(FsError::InvalidRelativePath {
            path: relative_path.to_owned(),
        });
    }

    let parent = clean_path.parent().unwrap_or_else(|| Path::new(""));
    let parent_path = resolve_existing_path(root, &path_to_string(parent))?;
    let candidate = parent_path.join(clean_path.file_name().ok_or_else(|| {
        FsError::InvalidRelativePath {
            path: relative_path.to_owned(),
        }
    })?);
    ensure_inside_workspace(root, &candidate, relative_path)?;
    Ok(candidate)
}

fn clean_relative_path(relative_path: &str) -> Result<PathBuf, FsError> {
    let path = Path::new(relative_path);
    let mut clean = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => clean.push(part),
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(FsError::InvalidRelativePath {
                    path: relative_path.to_owned(),
                });
            }
        }
    }

    Ok(clean)
}

fn ensure_inside_workspace(root: &Path, path: &Path, original_path: &str) -> Result<(), FsError> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(FsError::PathOutsideWorkspace {
            path: original_path.to_owned(),
        })
    }
}

fn entry_for_path(root: &Path, path: &Path) -> Result<FsEntry, FsError> {
    let canonical_path = path.canonicalize().map_err(|_| FsError::PathNotFound {
        path: path.display().to_string(),
    })?;
    ensure_inside_workspace(root, &canonical_path, &path.display().to_string())?;
    let metadata = metadata_for(&canonical_path, "read entry metadata")?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = if metadata.is_dir() {
        FsEntryKind::Directory
    } else {
        FsEntryKind::File
    };

    Ok(FsEntry {
        name,
        path: relative_path_string(root, &canonical_path)?,
        kind,
        size_bytes: metadata.is_file().then_some(metadata.len()),
        modified_ms: metadata.modified().ok().and_then(|modified| {
            modified
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        }),
    })
}

pub(crate) fn relative_path_string(root: &Path, path: &Path) -> Result<String, FsError> {
    let relative_path = path.strip_prefix(root).map_err(|_| FsError::PathOutsideWorkspace {
        path: path.display().to_string(),
    })?;
    Ok(path_to_string(relative_path))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn metadata_for(path: &Path, action: &'static str) -> Result<fs::Metadata, FsError> {
    fs::metadata(path).map_err(|source| FsError::Io { action, source })
}
