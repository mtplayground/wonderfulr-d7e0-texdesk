use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const DATABASE_FILE_NAME: &str = "texdesk.sqlite3";
const INITIAL_MIGRATION: &str = include_str!("../../migrations/001_initial_store.sql");

#[derive(Debug)]
pub enum StoreError {
    AppDataDir(tauri::Error),
    CreateDataDir {
        path: PathBuf,
        source: std::io::Error,
    },
    OpenDatabase {
        path: PathBuf,
        source: rusqlite::Error,
    },
    Migration(rusqlite::Error),
    Query(rusqlite::Error),
}

impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AppDataDir(_) => "store_app_data_dir",
            Self::CreateDataDir { .. } => "store_create_data_dir",
            Self::OpenDatabase { .. } => "store_open_database",
            Self::Migration(_) => "store_migration",
            Self::Query(_) => "store_query",
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppDataDir(source) => {
                write!(formatter, "could not resolve application data directory: {source}")
            }
            Self::CreateDataDir { path, source } => {
                write!(
                    formatter,
                    "could not create application data directory {}: {source}",
                    path.display()
                )
            }
            Self::OpenDatabase { path, source } => {
                write!(formatter, "could not open SQLite store {}: {source}", path.display())
            }
            Self::Migration(source) => {
                write!(formatter, "could not apply SQLite migrations: {source}")
            }
            Self::Query(source) => write!(formatter, "could not query SQLite store: {source}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AppDataDir(source) => Some(source),
            Self::CreateDataDir { source, .. } => Some(source),
            Self::OpenDatabase { source, .. } => Some(source),
            Self::Migration(source) | Self::Query(source) => Some(source),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Store {
    database_path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreStatus {
    pub database_path: String,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub last_workspace_root: Option<String>,
    pub last_open_file: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub workspace_root: String,
    pub last_opened_at: String,
}

impl Store {
    pub fn initialize(app_handle: &AppHandle) -> Result<Self, StoreError> {
        let data_dir = app_handle.path().app_data_dir().map_err(StoreError::AppDataDir)?;
        fs::create_dir_all(&data_dir).map_err(|source| StoreError::CreateDataDir {
            path: data_dir.clone(),
            source,
        })?;

        let store = Self {
            database_path: data_dir.join(DATABASE_FILE_NAME),
        };
        store.apply_migrations()?;
        Ok(store)
    }

    pub fn status(&self) -> Result<StoreStatus, StoreError> {
        Ok(StoreStatus {
            database_path: self.database_path.display().to_string(),
            schema_version: self.schema_version()?,
        })
    }

    pub fn workspace_state(&self) -> Result<WorkspaceState, StoreError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT last_workspace_root, last_open_file FROM workspace_state WHERE id = 1",
                [],
                |row| {
                    Ok(WorkspaceState {
                        last_workspace_root: row.get(0)?,
                        last_open_file: row.get(1)?,
                    })
                },
            )
            .map_err(StoreError::Query)
    }

    pub fn remember_workspace_root(
        &self,
        workspace_root: &str,
    ) -> Result<WorkspaceState, StoreError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO recent_projects (workspace_root, last_opened_at)
                 VALUES (?1, CURRENT_TIMESTAMP)
                 ON CONFLICT(workspace_root)
                 DO UPDATE SET last_opened_at = CURRENT_TIMESTAMP",
                params![workspace_root],
            )
            .map_err(StoreError::Query)?;
        connection
            .execute(
                "UPDATE workspace_state
                 SET last_workspace_root = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE id = 1",
                params![workspace_root],
            )
            .map_err(StoreError::Query)?;
        self.workspace_state()
    }

    pub fn remember_open_file(
        &self,
        workspace_root: &str,
        path: &str,
    ) -> Result<WorkspaceState, StoreError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO recent_projects (workspace_root, last_opened_at)
                 VALUES (?1, CURRENT_TIMESTAMP)
                 ON CONFLICT(workspace_root)
                 DO UPDATE SET last_opened_at = CURRENT_TIMESTAMP",
                params![workspace_root],
            )
            .map_err(StoreError::Query)?;
        connection
            .execute(
                "UPDATE workspace_state
                 SET last_workspace_root = ?1,
                     last_open_file = ?2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = 1",
                params![workspace_root, path],
            )
            .map_err(StoreError::Query)?;
        self.workspace_state()
    }

    pub fn recent_projects(&self, limit: i64) -> Result<Vec<RecentProject>, StoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_root, last_opened_at
                 FROM recent_projects
                 ORDER BY last_opened_at DESC
                 LIMIT ?1",
            )
            .map_err(StoreError::Query)?;
        let rows = statement
            .query_map(params![limit.max(1)], |row| {
                Ok(RecentProject {
                    workspace_root: row.get(0)?,
                    last_opened_at: row.get(1)?,
                })
            })
            .map_err(StoreError::Query)?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row.map_err(StoreError::Query)?);
        }
        Ok(projects)
    }

    fn connection(&self) -> Result<Connection, StoreError> {
        open_database(&self.database_path)
    }

    fn apply_migrations(&self) -> Result<(), StoreError> {
        let connection = self.connection()?;
        connection
            .execute_batch(INITIAL_MIGRATION)
            .map_err(StoreError::Migration)
    }

    fn schema_version(&self) -> Result<i64, StoreError> {
        let connection = self.connection()?;
        let version = connection
            .query_row(
                "SELECT MAX(version) FROM schema_migrations",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(StoreError::Query)?
            .flatten()
            .unwrap_or(0);

        Ok(version)
    }
}

fn open_database(path: &Path) -> Result<Connection, StoreError> {
    let connection = Connection::open(path).map_err(|source| StoreError::OpenDatabase {
        path: path.to_path_buf(),
        source,
    })?;

    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(StoreError::Query)?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(StoreError::Query)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(StoreError::Query)?;

    Ok(connection)
}
