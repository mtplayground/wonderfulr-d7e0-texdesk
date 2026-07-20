use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fmt;
use std::fs as stdfs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config::AppConfig;
use crate::fs::{self, FsError};

#[derive(Debug)]
pub enum CompileError {
    Fs(FsError),
    FileExpected { path: String },
    InvalidSourceName { path: String },
    ToolchainUnavailable,
    Io {
        action: &'static str,
        source: std::io::Error,
    },
    ProcessFailed {
        tool: String,
        status: String,
        log: String,
    },
    PdfNotProduced { path: String, log: String },
}

impl CompileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Fs(error) => error.code(),
            Self::FileExpected { .. } => "compile_file_expected",
            Self::InvalidSourceName { .. } => "compile_invalid_source_name",
            Self::ToolchainUnavailable => "compile_toolchain_unavailable",
            Self::Io { .. } => "compile_io",
            Self::ProcessFailed { .. } => "compile_process_failed",
            Self::PdfNotProduced { .. } => "compile_pdf_not_produced",
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fs(error) => write!(formatter, "{error}"),
            Self::FileExpected { path } => write!(formatter, "LaTeX source must be a .tex file: {path}"),
            Self::InvalidSourceName { path } => {
                write!(formatter, "LaTeX source has no valid file name: {path}")
            }
            Self::ToolchainUnavailable => write!(
                formatter,
                "no LaTeX compiler found; install latexmk, pdflatex, or xelatex"
            ),
            Self::Io { action, source } => write!(formatter, "{action} failed: {source}"),
            Self::ProcessFailed { tool, status, log } => {
                write!(formatter, "{tool} exited with {status}\n{log}")
            }
            Self::PdfNotProduced { path, log } => {
                write!(formatter, "compile finished but no PDF was produced at {path}\n{log}")
            }
        }
    }
}

impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<FsError> for CompileError {
    fn from(error: FsError) -> Self {
        Self::Fs(error)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileDocumentRequest {
    pub workspace_root: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    pub pdf_path: String,
    pub log: String,
    pub toolchain: CompileToolchain,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileToolchain {
    pub strategy: CompileStrategy,
    pub engine: String,
    pub bibliography_tool: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompileStrategy {
    Latexmk,
    ManualPasses,
}

#[derive(Clone, Debug)]
struct Tool {
    name: String,
    path: PathBuf,
}

pub fn compile_document(request: CompileDocumentRequest) -> Result<CompileResult, CompileError> {
    let root = fs::canonical_workspace_root(&request.workspace_root)?;
    let source_path = fs::resolve_existing_path(&root, &request.path)?;
    if source_path
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("tex")
    {
        return Err(CompileError::FileExpected { path: request.path });
    }

    let source_dir = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.clone());
    let source_file = source_path
        .file_name()
        .map(OsString::from)
        .ok_or_else(|| CompileError::InvalidSourceName {
            path: source_path.display().to_string(),
        })?;
    let source_stem = source_path
        .file_stem()
        .map(OsString::from)
        .ok_or_else(|| CompileError::InvalidSourceName {
            path: source_path.display().to_string(),
        })?;
    let pdf_path = source_path.with_extension("pdf");
    let config = AppConfig::from_env();
    let toolchain_path = config.latex_toolchain_path.as_deref();

    if let Some(latexmk) = find_tool("latexmk", toolchain_path) {
        return run_latexmk(&root, &source_dir, &source_file, &pdf_path, latexmk);
    }

    let engine = find_tool("pdflatex", toolchain_path)
        .or_else(|| find_tool("xelatex", toolchain_path))
        .ok_or(CompileError::ToolchainUnavailable)?;
    run_manual_passes(
        &root,
        &source_dir,
        &source_file,
        &source_stem,
        &source_path,
        &pdf_path,
        engine,
        toolchain_path,
    )
}

fn run_latexmk(
    root: &Path,
    source_dir: &Path,
    source_file: &OsString,
    pdf_path: &Path,
    latexmk: Tool,
) -> Result<CompileResult, CompileError> {
    let args = vec![
        OsString::from("-pdf"),
        OsString::from("-interaction=nonstopmode"),
        OsString::from("-halt-on-error"),
        OsString::from("-file-line-error"),
        source_file.clone(),
    ];
    let run = run_tool(&latexmk, &args, source_dir)?;
    ensure_success(&latexmk.name, &run)?;
    let log = run.log;
    let pdf_relative_path = ensure_pdf(root, pdf_path, &log)?;

    Ok(CompileResult {
        pdf_path: pdf_relative_path,
        log,
        toolchain: CompileToolchain {
            strategy: CompileStrategy::Latexmk,
            engine: latexmk.name,
            bibliography_tool: None,
        },
    })
}

fn run_manual_passes(
    root: &Path,
    source_dir: &Path,
    source_file: &OsString,
    source_stem: &OsString,
    source_path: &Path,
    pdf_path: &Path,
    engine: Tool,
    toolchain_path: Option<&str>,
) -> Result<CompileResult, CompileError> {
    let mut combined_log = String::new();
    let engine_args = vec![
        OsString::from("-interaction=nonstopmode"),
        OsString::from("-halt-on-error"),
        OsString::from("-file-line-error"),
        source_file.clone(),
    ];

    append_run(&mut combined_log, run_tool(&engine, &engine_args, source_dir)?)?;

    let bibliography_tool = if needs_bibliography_pass(source_path, source_dir, source_stem) {
        run_bibliography_pass(source_dir, source_stem, toolchain_path, &mut combined_log)?
    } else {
        None
    };

    append_run(&mut combined_log, run_tool(&engine, &engine_args, source_dir)?)?;
    append_run(&mut combined_log, run_tool(&engine, &engine_args, source_dir)?)?;

    let pdf_relative_path = ensure_pdf(root, pdf_path, &combined_log)?;
    Ok(CompileResult {
        pdf_path: pdf_relative_path,
        log: combined_log,
        toolchain: CompileToolchain {
            strategy: CompileStrategy::ManualPasses,
            engine: engine.name,
            bibliography_tool,
        },
    })
}

fn run_bibliography_pass(
    source_dir: &Path,
    source_stem: &OsString,
    toolchain_path: Option<&str>,
    combined_log: &mut String,
) -> Result<Option<String>, CompileError> {
    let aux_path = source_dir.join(Path::new(source_stem)).with_extension("aux");
    let aux_contents = stdfs::read_to_string(&aux_path).unwrap_or_default();
    let preferred_tool_name = if aux_contents.contains("\\abx@aux@cite")
        || aux_contents.contains("\\abx@aux@refcontext")
    {
        "biber"
    } else {
        "bibtex"
    };

    let bibliography_tool = find_tool(preferred_tool_name, toolchain_path)
        .or_else(|| find_tool("bibtex", toolchain_path))
        .or_else(|| find_tool("biber", toolchain_path));
    let Some(tool) = bibliography_tool else {
        combined_log.push_str("\n[texdesk] Bibliography references detected, but neither bibtex nor biber was found.\n");
        return Ok(None);
    };

    let run = run_tool(&tool, &[source_stem.clone()], source_dir)?;
    append_run(combined_log, run)?;
    Ok(Some(tool.name))
}

fn needs_bibliography_pass(source_path: &Path, source_dir: &Path, source_stem: &OsString) -> bool {
    let source_contains_bibliography = stdfs::read_to_string(source_path)
        .map(|contents| {
            contents.contains("\\cite")
                || contents.contains("\\nocite")
                || contents.contains("\\bibliography")
                || contents.contains("\\addbibresource")
        })
        .unwrap_or(false);
    if source_contains_bibliography {
        return true;
    }

    let aux_path = source_dir.join(Path::new(source_stem)).with_extension("aux");
    stdfs::read_to_string(aux_path)
        .map(|contents| {
            contents.contains("\\citation")
                || contents.contains("\\bibdata")
                || contents.contains("\\abx@aux@cite")
        })
        .unwrap_or(false)
}

struct ToolRun {
    tool_name: String,
    status: std::process::ExitStatus,
    log: String,
}

fn append_run(combined_log: &mut String, run: ToolRun) -> Result<(), CompileError> {
    if !run.status.success() {
        let mut log = combined_log.clone();
        log.push_str(&run.log);
        return Err(CompileError::ProcessFailed {
            tool: run.tool_name,
            status: status_label(run.status),
            log,
        });
    }

    combined_log.push_str(&run.log);
    Ok(())
}

fn ensure_success(tool_name: &str, run: &ToolRun) -> Result<(), CompileError> {
    if run.status.success() {
        Ok(())
    } else {
        Err(CompileError::ProcessFailed {
            tool: tool_name.to_owned(),
            status: status_label(run.status),
            log: run.log.clone(),
        })
    }
}

fn ensure_pdf(root: &Path, pdf_path: &Path, log: &str) -> Result<String, CompileError> {
    if pdf_path.is_file() {
        fs::relative_path_string(root, pdf_path).map_err(CompileError::from)
    } else {
        Err(CompileError::PdfNotProduced {
            path: pdf_path.display().to_string(),
            log: log.to_owned(),
        })
    }
}

fn run_tool(tool: &Tool, args: &[OsString], cwd: &Path) -> Result<ToolRun, CompileError> {
    let output = Command::new(&tool.path)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|source| CompileError::Io {
            action: "run LaTeX process",
            source,
        })?;
    Ok(ToolRun {
        tool_name: tool.name.clone(),
        status: output.status,
        log: format_run_log(tool, args, cwd, &output),
    })
}

fn format_run_log(tool: &Tool, args: &[OsString], cwd: &Path, output: &Output) -> String {
    let command_line = std::iter::once(tool.name.clone())
        .chain(args.iter().map(|argument| argument.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    format!(
        "\n$ {command_line}\n[working directory] {}\n[status] {}\n{}{}",
        cwd.display(),
        status_label(output.status),
        stdout,
        stderr
    )
}

fn status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| "terminated by signal".to_owned())
}

fn find_tool(name: &str, toolchain_path: Option<&str>) -> Option<Tool> {
    if let Some(path) = toolchain_path.and_then(|value| find_tool_in_override(name, value)) {
        return Some(Tool {
            name: name.to_owned(),
            path,
        });
    }

    let path = PathBuf::from(name);
    command_version_works(&path).then(|| Tool {
        name: name.to_owned(),
        path,
    })
}

fn find_tool_in_override(name: &str, override_path: &str) -> Option<PathBuf> {
    let path = Path::new(override_path);
    if path.is_dir() {
        let candidate = path.join(binary_name(name));
        return command_version_works(&candidate).then_some(candidate);
    }

    if path.is_file()
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == name)
        && command_version_works(path)
    {
        return Some(path.to_path_buf());
    }

    None
}

fn command_version_works(path: &Path) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}
