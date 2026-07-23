use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
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
                "no LaTeX compiler found; install latexmk, pdflatex, xelatex, or lualatex"
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
        match run_latexmk(&root, &source_dir, &source_file, &pdf_path, latexmk) {
            Ok(result) => return Ok(result),
            Err(error) if is_miktex_latexmk_missing_perl_failure(&error) => {
                let fallback_log = latexmk_missing_perl_fallback_log(&error);
                let Some(engine) = find_manual_engine(toolchain_path) else {
                    return Err(error);
                };
                let fallback_engine_name = engine.name.clone();
                let mut result = run_manual_passes(
                    &root,
                    &source_dir,
                    &source_file,
                    &source_stem,
                    &source_path,
                    &pdf_path,
                    engine,
                    toolchain_path,
                )?;
                result.log = format!(
                    "{fallback_log}\n[texdesk] latexmk failed because MiKTeX could not find Perl; falling back to native engine {fallback_engine_name}.\n{}",
                    result.log
                );
                return Ok(result);
            }
            Err(error) => return Err(error),
        }
    }

    let engine = find_manual_engine(toolchain_path).ok_or(CompileError::ToolchainUnavailable)?;
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

fn find_manual_engine(toolchain_path: Option<&str>) -> Option<Tool> {
    ["pdflatex", "xelatex", "lualatex"]
        .into_iter()
        .find_map(|name| find_tool(name, toolchain_path))
}

fn is_miktex_latexmk_missing_perl_failure(error: &CompileError) -> bool {
    let CompileError::ProcessFailed { tool, log, .. } = error else {
        return false;
    };
    if !tool.eq_ignore_ascii_case("latexmk") {
        return false;
    }

    log_indicates_miktex_latexmk_missing_perl(log)
}

fn latexmk_missing_perl_fallback_log(error: &CompileError) -> String {
    match error {
        CompileError::ProcessFailed { log, .. } => log.clone(),
        other => other.to_string(),
    }
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
    tool_is_detectable(name, &path).then(|| Tool {
        name: name.to_owned(),
        path,
    })
    .or_else(|| {
        find_tool_in_known_paths(name).map(|path| Tool {
            name: name.to_owned(),
            path,
        })
    })
}

fn find_tool_in_override(name: &str, override_path: &str) -> Option<PathBuf> {
    let path = Path::new(override_path);
    if path.is_dir() {
        return candidate_binary_names(name)
            .into_iter()
            .map(|binary| path.join(binary))
            .find(|candidate| tool_is_detectable(name, candidate));
    }

    if path.is_file()
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == name)
        && tool_is_detectable(name, path)
    {
        return Some(path.to_path_buf());
    }

    None
}

fn tool_is_detectable(name: &str, path: &Path) -> bool {
    let Some(output) = command_version_output(path) else {
        return false;
    };

    output.status.success()
        || (name.eq_ignore_ascii_case("latexmk")
            && output_indicates_miktex_latexmk_missing_perl(&output))
}

fn command_version_output(path: &Path) -> Option<Output> {
    Command::new(path).arg("--version").output().ok()
}

fn output_indicates_miktex_latexmk_missing_perl(output: &Output) -> bool {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    log_indicates_miktex_latexmk_missing_perl(&format!("{stdout}{stderr}"))
}

fn log_indicates_miktex_latexmk_missing_perl(log: &str) -> bool {
    let lower_log = log.to_ascii_lowercase();
    lower_log.contains("miktex")
        && lower_log.contains("script engine")
        && lower_log.contains("perl")
}

fn find_tool_in_known_paths(name: &str) -> Option<PathBuf> {
    get_known_toolchain_directories()
        .into_iter()
        .flat_map(|directory| {
            candidate_binary_names(name)
                .into_iter()
                .map(move |binary| directory.join(binary))
        })
        .find(|candidate| tool_is_detectable(name, candidate))
}

pub(crate) fn detect_latex_toolchain_path() -> Option<String> {
    const TOOLCHAIN_SENTINELS: &[&str] =
        &["latexmk", "pdflatex", "xelatex", "lualatex", "tectonic"];

    get_known_toolchain_directories()
        .into_iter()
        .find(|directory| {
            TOOLCHAIN_SENTINELS
                .iter()
                .any(|&tool| directory_contains_detectable_tool(&directory, tool))
        })
        .map(|directory| directory.display().to_string())
}

fn directory_contains_detectable_tool(directory: &Path, name: &str) -> bool {
    candidate_binary_names(name)
        .into_iter()
        .map(|binary| directory.join(binary))
        .any(|candidate| tool_is_detectable(name, &candidate))
}

fn get_known_toolchain_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let mut seen = HashSet::new();

    #[cfg(test)]
    append_test_toolchain_directories(&mut directories);
    append_platform_toolchain_directories(&mut directories);

    directories
        .into_iter()
        .filter(|directory| directory.is_dir())
        .filter(|directory| seen.insert(normalize_known_directory_key(directory)))
        .collect()
}

fn normalize_known_directory_key(directory: &Path) -> String {
    directory.to_string_lossy().to_lowercase()
}

#[cfg(test)]
fn append_test_toolchain_directories(directories: &mut Vec<PathBuf>) {
    if let Some(path_dirs) = env::var_os("TEXDESK_TEST_KNOWN_TOOLCHAIN_DIRS") {
        directories.extend(env::split_paths(&path_dirs));
    }
}

#[cfg(target_os = "windows")]
fn append_platform_toolchain_directories(directories: &mut Vec<PathBuf>) {
    if let Some(local_app_data) = env_path("LOCALAPPDATA") {
        directories.push(
            local_app_data
                .join("Programs")
                .join("MiKTeX")
                .join("miktex")
                .join("bin")
                .join("x64"),
        );
        directories.push(
            local_app_data
                .join("Programs")
                .join("MiKTeX")
                .join("miktex")
                .join("bin"),
        );
        directories.push(local_app_data.join("Programs").join("Tectonic"));
        directories.push(local_app_data.join("Microsoft").join("WinGet").join("Links"));
    }

    for program_files_key in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(program_files) = env_path(program_files_key) {
            directories.push(
                program_files
                    .join("MiKTeX")
                    .join("miktex")
                    .join("bin")
                    .join("x64"),
            );
            directories.push(
                program_files
                    .join("MiKTeX")
                    .join("miktex")
                    .join("bin"),
            );
            directories.push(program_files.join("Tectonic"));
            directories.push(program_files.join("Tectonic").join("bin"));
        }
    }

    append_texlive_year_directories(directories, &PathBuf::from(r"C:\texlive"));

    if let Some(path_dirs) =
        env::var_os("PATH").map(|path| env::split_paths(&path).collect::<Vec<_>>())
    {
        directories.extend(path_dirs);
    }
}

#[cfg(target_os = "macos")]
fn append_platform_toolchain_directories(directories: &mut Vec<PathBuf>) {
    directories.push(PathBuf::from("/Library/TeX/texbin"));
    append_texlive_year_directories(directories, Path::new("/usr/local/texlive"));
    directories.push(PathBuf::from("/opt/homebrew/bin"));
    directories.push(PathBuf::from("/usr/local/bin"));
    directories.push(PathBuf::from("/opt/local/bin"));
    append_home_relative_directories(
        directories,
        &[
            ".cargo/bin",
            ".local/bin",
            "Library/Application Support/Tectonic",
        ],
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
fn append_platform_toolchain_directories(directories: &mut Vec<PathBuf>) {
    append_texlive_year_directories(directories, Path::new("/usr/local/texlive"));
    directories.push(PathBuf::from("/usr/local/bin"));
    directories.push(PathBuf::from("/usr/bin"));
    directories.push(PathBuf::from("/bin"));
    directories.push(PathBuf::from("/snap/bin"));
    directories.push(PathBuf::from("/opt/tectonic"));
    directories.push(PathBuf::from("/opt/tectonic/bin"));
    append_home_relative_directories(directories, &[".cargo/bin", ".local/bin"]);
}

#[cfg(not(any(unix, target_os = "windows")))]
fn append_platform_toolchain_directories(directories: &mut Vec<PathBuf>) {
    if let Some(path_dirs) =
        env::var_os("PATH").map(|path| env::split_paths(&path).collect::<Vec<_>>())
    {
        directories.extend(path_dirs);
    }
}

fn append_texlive_year_directories(directories: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = stdfs::read_dir(root) else {
        return;
    };

    let mut years = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let year = path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| {
                    name.len() == 4 && name.chars().all(|character| character.is_ascii_digit())
                })?
                .to_owned();
            Some((year, path))
        })
        .collect::<Vec<_>>();
    years.sort_by(|left, right| right.0.cmp(&left.0));

    for (_, year_dir) in years {
        append_texlive_bin_directories(directories, &year_dir);
    }
}

#[cfg(target_os = "windows")]
fn append_texlive_bin_directories(directories: &mut Vec<PathBuf>, year_dir: &Path) {
    directories.push(year_dir.join("bin").join("windows"));
}

#[cfg(target_os = "macos")]
fn append_texlive_bin_directories(directories: &mut Vec<PathBuf>, year_dir: &Path) {
    directories.push(year_dir.join("bin").join("universal-darwin"));
    directories.push(year_dir.join("bin").join("x86_64-darwin"));
    directories.push(year_dir.join("bin").join("aarch64-darwin"));
}

#[cfg(all(unix, not(target_os = "macos")))]
fn append_texlive_bin_directories(directories: &mut Vec<PathBuf>, year_dir: &Path) {
    directories.push(year_dir.join("bin").join("x86_64-linux"));
    directories.push(year_dir.join("bin").join("aarch64-linux"));
    directories.push(year_dir.join("bin").join(format!("{}-linux", env::consts::ARCH)));
}

#[cfg(not(any(unix, target_os = "windows")))]
fn append_texlive_bin_directories(_directories: &mut Vec<PathBuf>, _year_dir: &Path) {}

fn append_home_relative_directories(directories: &mut Vec<PathBuf>, relative_paths: &[&str]) {
    let Some(home) = env_path("HOME").or_else(|| env_path("USERPROFILE")) else {
        return;
    };

    directories.extend(relative_paths.iter().map(|relative| home.join(relative)));
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn candidate_binary_names(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
        ]
    } else {
        vec![binary_name(name)]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compile_document, detect_latex_toolchain_path, find_tool, CompileDocumentRequest,
        CompileError, CompileStrategy,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct TestWorkspace {
        root: PathBuf,
        previous_toolchain_path: Option<String>,
    }

    struct TempWorkspace {
        root: PathBuf,
    }

    struct FakeToolScript {
        unix: String,
        windows: String,
    }

    impl FakeToolScript {
        fn new(unix: impl Into<String>, windows: impl Into<String>) -> Self {
            Self {
                unix: unix.into(),
                windows: windows.into(),
            }
        }
    }

    struct EnvVarGuard {
        key: &'static str,
        previous_value: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set_path(key: &'static str, path: &Path) -> Self {
            let previous_value = std::env::var_os(key);
            std::env::set_var(key, path);
            Self {
                key,
                previous_value,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous_value {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock should be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("texdesk-{name}-{unique}"));
            fs::create_dir_all(&root).expect("create temporary workspace");
            Self { root }
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    impl TestWorkspace {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock should be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("texdesk-{name}-{unique}"));
            fs::create_dir_all(root.join("bin")).expect("create fake toolchain directory");
            let previous_toolchain_path = std::env::var("LATEX_TOOLCHAIN_PATH").ok();
            std::env::set_var("LATEX_TOOLCHAIN_PATH", root.join("bin"));

            Self {
                root,
                previous_toolchain_path,
            }
        }

        fn bin_dir(&self) -> PathBuf {
            self.root.join("bin")
        }

        fn write_source(&self, name: &str, contents: &str) {
            fs::write(self.root.join(name), contents).expect("write source");
        }

        fn write_tool(&self, name: &str, script: FakeToolScript) {
            write_tool_script(&self.bin_dir(), name, script);
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            match &self.previous_toolchain_path {
                Some(value) => std::env::set_var("LATEX_TOOLCHAIN_PATH", value),
                None => std::env::remove_var("LATEX_TOOLCHAIN_PATH"),
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_tool_script(directory: &Path, name: &str, script: FakeToolScript) -> PathBuf {
        let path = tool_script_path(directory, name);
        let body = if cfg!(windows) {
            script.windows
        } else {
            script.unix
        };
        write_executable(&path, &body);
        path
    }

    fn tool_script_path(directory: &Path, name: &str) -> PathBuf {
        if cfg!(windows) {
            directory.join(format!("{name}.cmd"))
        } else {
            directory.join(name)
        }
    }

    fn write_executable(path: &Path, body: &str) {
        fs::write(path, body).expect("write fake executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path)
                .expect("fake executable metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("chmod fake executable");
        }
    }

    fn successful_latex_script(label: &str) -> FakeToolScript {
        FakeToolScript::new(
            format!(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "{label} version"
  exit 0
fi
last=""
for arg in "$@"; do
  last="$arg"
done
stem="${{last%.tex}}"
echo "{label} compiling $last"
printf "\\relax\n" > "$stem.aux"
touch "$stem.pdf"
exit 0
"#,
            ),
            format!(
                r#"@echo off
if "%~1"=="--version" (
  echo {label} version
  exit /b 0
)
set "last="
:next_arg
if "%~1"=="" goto after_args
set "last=%~1"
shift
goto next_arg
:after_args
set "stem=%last:.tex=%"
echo {label} compiling %last%
> "%stem%.aux" echo \relax
type nul > "%stem%.pdf"
exit /b 0
"#,
            ),
        )
    }

    fn known_tool_script(label: &str) -> FakeToolScript {
        FakeToolScript::new(
            format!(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "{label} version"
  exit 0
fi
exit 1
"#,
            ),
            format!(
                r#"@echo off
if "%~1"=="--version" (
  echo {label} version
  exit /b 0
)
exit /b 1
"#,
            ),
        )
    }

    fn real_latex_toolchain_available() -> bool {
        let override_path = std::env::var("LATEX_TOOLCHAIN_PATH").ok();
        let toolchain_path = override_path.as_deref();

        find_tool("latexmk", toolchain_path).is_some()
            || find_tool("pdflatex", toolchain_path).is_some()
            || find_tool("xelatex", toolchain_path).is_some()
            || find_tool("lualatex", toolchain_path).is_some()
    }

    #[test]
    fn find_tool_discovers_executable_in_known_toolchain_directory() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TempWorkspace::new("known-toolchain");
        let known_bin = workspace.root.join("known-bin");
        fs::create_dir_all(&known_bin).expect("create known toolchain directory");
        let _known_dirs = EnvVarGuard::set_path("TEXDESK_TEST_KNOWN_TOOLCHAIN_DIRS", &known_bin);
        let tool_path =
            write_tool_script(&known_bin, "texdesk-known-tool", known_tool_script("known tool"));

        let tool = find_tool("texdesk-known-tool", None)
            .expect("known-directory executable should be detected");

        assert_eq!(tool.path, tool_path);
    }

    #[test]
    fn detect_latex_toolchain_path_returns_known_distribution_directory() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TempWorkspace::new("known-distribution");
        let known_bin = workspace.root.join("texlive").join("bin");
        fs::create_dir_all(&known_bin).expect("create known distribution directory");
        let _known_dirs = EnvVarGuard::set_path("TEXDESK_TEST_KNOWN_TOOLCHAIN_DIRS", &known_bin);
        write_tool_script(&known_bin, "pdflatex", known_tool_script("pdfTeX"));

        let detected = detect_latex_toolchain_path()
            .expect("known distribution directory should be detected");

        assert_eq!(detected, known_bin.display().to_string());
    }

    #[test]
    fn compile_prefers_latexmk_and_captures_log() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("latexmk");
        workspace.write_source(
            "main.tex",
            "\\documentclass{article}\\begin{document}Hi\\end{document}",
        );
        workspace.write_tool("latexmk", successful_latex_script("latexmk"));
        workspace.write_tool("pdflatex", successful_latex_script("pdflatex"));

        let result = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "main.tex".to_owned(),
        })
        .expect("latexmk compile should succeed");

        assert_eq!(result.pdf_path, "main.pdf");
        assert!(matches!(result.toolchain.strategy, CompileStrategy::Latexmk));
        assert_eq!(result.toolchain.engine, "latexmk");
        assert!(result.log.contains("$ latexmk -pdf"));
        assert!(result.log.contains("latexmk compiling main.tex"));
    }

    #[test]
    fn compile_falls_back_when_miktex_latexmk_is_missing_perl() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("latexmk-missing-perl");
        workspace.write_source(
            "main.tex",
            "\\documentclass{article}\\begin{document}Hi\\end{document}",
        );
        workspace.write_tool(
            "latexmk",
            FakeToolScript::new(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "MiKTeX could not find the script engine 'perl' which is required to execute 'latexmk'"
  exit 1
fi
echo "MiKTeX could not find the script engine 'perl' which is required to execute 'latexmk'"
exit 1
"#,
                r#"@echo off
if "%~1"=="--version" (
  echo MiKTeX could not find the script engine 'perl' which is required to execute 'latexmk'
  exit /b 1
)
echo MiKTeX could not find the script engine 'perl' which is required to execute 'latexmk'
exit /b 1
"#,
            ),
        );
        workspace.write_tool("pdflatex", successful_latex_script("pdflatex"));

        let result = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "main.tex".to_owned(),
        })
        .expect("MiKTeX latexmk missing Perl should fall back to pdflatex");

        assert_eq!(result.pdf_path, "main.pdf");
        assert!(matches!(
            result.toolchain.strategy,
            CompileStrategy::ManualPasses
        ));
        assert_eq!(result.toolchain.engine, "pdflatex");
        assert!(result.log.contains("MiKTeX could not find the script engine 'perl'"));
        assert!(result.log.contains("falling back to native engine pdflatex"));
        assert_eq!(result.log.matches("pdflatex compiling main.tex").count(), 3);
    }

    #[test]
    fn compile_does_not_fallback_for_regular_latexmk_failure() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("latexmk-regular-failure");
        workspace.write_source("broken.tex", "\\documentclass{article}");
        workspace.write_tool(
            "latexmk",
            FakeToolScript::new(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "Latexmk version"
  exit 0
fi
echo "latexmk found a document error"
exit 12
"#,
                r#"@echo off
if "%~1"=="--version" (
  echo Latexmk version
  exit /b 0
)
echo latexmk found a document error
exit /b 12
"#,
            ),
        );
        workspace.write_tool("pdflatex", successful_latex_script("pdflatex"));

        let error = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "broken.tex".to_owned(),
        })
        .expect_err("non-MiKTeX latexmk failures should not be retried");

        match error {
            CompileError::ProcessFailed { tool, status, log } => {
                assert_eq!(tool, "latexmk");
                assert_eq!(status, "exit code 12");
                assert!(log.contains("latexmk found a document error"));
                assert!(!log.contains("pdflatex compiling broken.tex"));
            }
            other => panic!("expected latexmk process failure, got {other:?}"),
        }
    }

    #[test]
    fn compile_uses_lualatex_when_other_native_engines_are_unavailable() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("lualatex");
        workspace.write_source(
            "main.tex",
            "\\documentclass{article}\\begin{document}Hi\\end{document}",
        );
        workspace.write_tool("lualatex", successful_latex_script("lualatex"));

        let result = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "main.tex".to_owned(),
        })
        .expect("manual compile should use lualatex when pdflatex and xelatex are unavailable");

        assert_eq!(result.pdf_path, "main.pdf");
        assert!(matches!(
            result.toolchain.strategy,
            CompileStrategy::ManualPasses
        ));
        assert_eq!(result.toolchain.engine, "lualatex");
        assert_eq!(result.log.matches("lualatex compiling main.tex").count(), 3);
    }

    #[test]
    fn compile_falls_back_to_manual_passes_with_bibliography() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("manual-bib");
        workspace.write_source(
            "paper.tex",
            "\\documentclass{article}\\begin{document}\\cite{key}\\bibliography{references}\\end{document}",
        );
        workspace.write_tool("pdflatex", successful_latex_script("pdflatex"));
        workspace.write_tool(
            "bibtex",
            FakeToolScript::new(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "bibtex version"
  exit 0
fi
echo "bibtex compiling $1"
touch "$1.bbl"
exit 0
"#,
                r#"@echo off
if "%~1"=="--version" (
  echo bibtex version
  exit /b 0
)
echo bibtex compiling %~1
type nul > "%~1.bbl"
exit /b 0
"#,
            ),
        );

        let result = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "paper.tex".to_owned(),
        })
        .expect("manual compile should succeed");

        assert_eq!(result.pdf_path, "paper.pdf");
        assert!(matches!(
            result.toolchain.strategy,
            CompileStrategy::ManualPasses
        ));
        assert_eq!(result.toolchain.engine, "pdflatex");
        assert_eq!(result.toolchain.bibliography_tool.as_deref(), Some("bibtex"));
        assert_eq!(result.log.matches("pdflatex compiling paper.tex").count(), 3);
        assert!(result.log.contains("bibtex compiling paper"));
    }

    #[test]
    fn compile_error_includes_failing_process_log() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        let workspace = TestWorkspace::new("failure-log");
        workspace.write_source("broken.tex", "\\documentclass{article}");
        workspace.write_tool(
            "pdflatex",
            FakeToolScript::new(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "pdflatex version"
  exit 0
fi
echo "fatal compile output"
echo "fatal compile error" >&2
exit 2
"#,
                r#"@echo off
if "%~1"=="--version" (
  echo pdflatex version
  exit /b 0
)
echo fatal compile output
1>&2 echo fatal compile error
exit /b 2
"#,
            ),
        );

        let error = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "broken.tex".to_owned(),
        })
        .expect_err("compile should fail");

        match error {
            CompileError::ProcessFailed { tool, status, log } => {
                assert_eq!(tool, "pdflatex");
                assert_eq!(status, "exit code 2");
                assert!(log.contains("fatal compile output"));
                assert!(log.contains("fatal compile error"));
            }
            other => panic!("expected process failure, got {other:?}"),
        }
    }

    #[test]
    #[ignore = "requires a real local LaTeX installation on PATH or LATEX_TOOLCHAIN_PATH"]
    fn e2e_template_edit_compile_preview_pdf() {
        let _guard = TEST_ENV_LOCK.lock().expect("lock test environment");
        if !real_latex_toolchain_available() {
            panic!(
                "end-to-end compile test requires latexmk, pdflatex, or xelatex on PATH or LATEX_TOOLCHAIN_PATH"
            );
        }

        let workspace = TempWorkspace::new("e2e-flow");
        let course_dir = workspace.root.join("calculus-101");
        fs::create_dir_all(&course_dir).expect("create course directory");

        let template = r#"\documentclass{article}
\title{Template Flow}
\author{TexDesk E2E}
\date{\today}

\begin{document}
\maketitle

\section*{Assignment}
Template placeholder.

\end{document}
"#;
        let edited = template.replace(
            "Template placeholder.",
            "Edited end-to-end content with $a^2 + b^2 = c^2$.",
        );
        fs::write(course_dir.join("assignment.tex"), edited).expect("write edited template file");

        let result = compile_document(CompileDocumentRequest {
            workspace_root: workspace.root.display().to_string(),
            path: "calculus-101/assignment.tex".to_owned(),
        })
        .expect("real LaTeX compile should succeed");

        assert_eq!(result.pdf_path, "calculus-101/assignment.pdf");
        assert!(result.log.contains("$ "));

        let pdf_bytes = fs::read(workspace.root.join(&result.pdf_path)).expect("read compiled PDF");
        assert!(
            pdf_bytes.starts_with(b"%PDF"),
            "compiled output should be a PDF that the preview pane can render"
        );
    }
}
