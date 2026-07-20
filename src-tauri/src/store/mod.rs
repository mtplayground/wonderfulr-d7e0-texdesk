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
    TemplateNotFound { id: String },
}

impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AppDataDir(_) => "store_app_data_dir",
            Self::CreateDataDir { .. } => "store_create_data_dir",
            Self::OpenDatabase { .. } => "store_open_database",
            Self::Migration(_) => "store_migration",
            Self::Query(_) => "store_query",
            Self::TemplateNotFound { .. } => "store_template_not_found",
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
            Self::TemplateNotFound { id } => write!(formatter, "template was not found: {id}"),
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
            Self::TemplateNotFound { .. } => None,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub main_file_name: String,
    pub body: String,
    pub bibliography: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

struct DefaultTemplate {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
    main_file_name: &'static str,
    body: &'static str,
    bibliography: Option<&'static str>,
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

    pub fn templates(&self) -> Result<Vec<Template>, StoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id,
                        name,
                        description,
                        category,
                        main_file_name,
                        body,
                        bibliography,
                        is_default,
                        created_at,
                        updated_at
                 FROM templates
                 ORDER BY is_default DESC, name ASC",
            )
            .map_err(StoreError::Query)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Template {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    category: row.get(3)?,
                    main_file_name: row.get(4)?,
                    body: row.get(5)?,
                    bibliography: row.get(6)?,
                    is_default: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(StoreError::Query)?;
        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(StoreError::Query)?);
        }
        Ok(templates)
    }

    pub fn template(&self, id: &str) -> Result<Template, StoreError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id,
                        name,
                        description,
                        category,
                        main_file_name,
                        body,
                        bibliography,
                        is_default,
                        created_at,
                        updated_at
                 FROM templates
                 WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Template {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        category: row.get(3)?,
                        main_file_name: row.get(4)?,
                        body: row.get(5)?,
                        bibliography: row.get(6)?,
                        is_default: row.get::<_, i64>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::Query)?
            .ok_or_else(|| StoreError::TemplateNotFound { id: id.to_owned() })
    }

    fn connection(&self) -> Result<Connection, StoreError> {
        open_database(&self.database_path)
    }

    fn apply_migrations(&self) -> Result<(), StoreError> {
        let connection = self.connection()?;
        connection
            .execute_batch(INITIAL_MIGRATION)
            .map_err(StoreError::Migration)?;
        ensure_template_schema(&connection).map_err(StoreError::Migration)?;
        seed_default_templates(&connection).map_err(StoreError::Migration)?;
        Ok(())
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

fn ensure_template_schema(connection: &Connection) -> Result<(), rusqlite::Error> {
    if !table_has_column(connection, "templates", "category")? {
        connection.execute(
            "ALTER TABLE templates ADD COLUMN category TEXT NOT NULL DEFAULT 'general'",
            [],
        )?;
    }
    if !table_has_column(connection, "templates", "main_file_name")? {
        connection.execute(
            "ALTER TABLE templates ADD COLUMN main_file_name TEXT NOT NULL DEFAULT 'main.tex'",
            [],
        )?;
    }
    if !table_has_column(connection, "templates", "bibliography")? {
        connection.execute("ALTER TABLE templates ADD COLUMN bibliography TEXT", [])?;
    }
    if !table_has_column(connection, "templates", "is_default")? {
        connection.execute(
            "ALTER TABLE templates ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1))",
            [],
        )?;
    }
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_templates_category ON templates (category, name)",
        [],
    )?;
    Ok(())
}

fn table_has_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, rusqlite::Error> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;

    for row in rows {
        if row? == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

fn seed_default_templates(connection: &Connection) -> Result<(), rusqlite::Error> {
    for template in DEFAULT_TEMPLATES {
        connection.execute(
            "INSERT INTO templates (
                id,
                name,
                description,
                category,
                main_file_name,
                body,
                bibliography,
                is_default,
                created_at,
                updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO NOTHING",
            params![
                template.id,
                template.name,
                template.description,
                template.category,
                template.main_file_name,
                template.body,
                template.bibliography,
            ],
        )?;
    }
    Ok(())
}

const DEFAULT_TEMPLATES: &[DefaultTemplate] = &[
    DefaultTemplate {
        id: "math-problem-set",
        name: "Math Problem Set",
        description: "A concise homework layout with numbered problems, theorem notation, and aligned equations.",
        category: "coursework",
        main_file_name: "problem-set.tex",
        body: r#"\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath,amssymb,amsthm}

\title{Problem Set Title}
\author{Student Name}
\date{\today}

\newtheorem*{claim}{Claim}

\begin{document}
\maketitle

\section*{Problem 1}
State the problem in your own words before solving it.

\begin{claim}
For every integer $n \geq 1$,
\[
  \sum_{k=1}^{n} k = \frac{n(n+1)}{2}.
\]
\end{claim}

\begin{proof}
Use induction on $n$. The base case is immediate. For the induction step,
\[
  \sum_{k=1}^{n+1} k
  = \frac{n(n+1)}{2} + (n+1)
  = \frac{(n+1)(n+2)}{2}.
\]
\end{proof}

\section*{Problem 2}
Add the next solution here.

\end{document}
"#,
        bibliography: None,
    },
    DefaultTemplate {
        id: "cited-paper-with-bibliography",
        name: "Cited Paper",
        description: "A short academic paper starter with citations and a companion BibTeX bibliography.",
        category: "paper",
        main_file_name: "paper.tex",
        body: r#"\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath}

\title{Paper Title}
\author{Author Name}
\date{\today}

\begin{document}
\maketitle

\begin{abstract}
Summarize the question, method, and main result in one compact paragraph.
\end{abstract}

\section{Introduction}
Introduce the topic and motivate the research question. Knuth's work on TeX
remains a useful reference point for high-quality technical typesetting
\cite{knuth1984texbook}.

\section{Method}
Describe the materials, assumptions, and analysis plan. Keep notation explicit:
\[
  y = \beta_0 + \beta_1 x + \epsilon.
\]

\section{Results}
Report the main findings and connect them back to the introduction.

\section{Conclusion}
State the takeaway and identify the next question.

\bibliographystyle{plain}
\bibliography{references}

\end{document}
"#,
        bibliography: Some(
            r#"@book{knuth1984texbook,
  author = {Donald E. Knuth},
  title = {The TeXbook},
  year = {1984},
  publisher = {Addison-Wesley},
  address = {Reading, Massachusetts}
}
"#,
        ),
    },
    DefaultTemplate {
        id: "figure-table-report",
        name: "Figure and Table Report",
        description: "A lab or project report scaffold with a figure placeholder, summary table, and conclusion.",
        category: "report",
        main_file_name: "report.tex",
        body: r#"\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}

\title{Report Title}
\author{Author Name}
\date{\today}

\begin{document}
\maketitle

\section{Overview}
Describe the objective, context, and criteria for interpreting the results.

\section{Results}
Figure~\ref{fig:placeholder} reserves space for the main visual, and
Table~\ref{tab:summary} summarizes the key measurements.

\begin{figure}[h]
  \centering
  \fbox{\rule{0pt}{1.6in}\rule{0.82\linewidth}{0pt}}
  \caption{Replace this placeholder with the primary figure.}
  \label{fig:placeholder}
\end{figure}

\begin{table}[h]
  \centering
  \caption{Summary measurements}
  \label{tab:summary}
  \begin{tabular}{lrr}
    \hline
    Condition & Mean & Standard Deviation \\
    \hline
    Baseline & 12.4 & 1.3 \\
    Treatment & 15.8 & 1.1 \\
    Follow-up & 14.9 & 1.5 \\
    \hline
  \end{tabular}
\end{table}

\section{Discussion}
Explain the implications, limitations, and next steps.

\end{document}
"#,
        bibliography: None,
    },
];
