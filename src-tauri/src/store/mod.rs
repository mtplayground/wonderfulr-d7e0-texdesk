use rusqlite::{Connection, OptionalExtension};
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
