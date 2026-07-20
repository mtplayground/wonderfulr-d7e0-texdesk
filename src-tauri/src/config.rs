use serde::Serialize;
use std::env;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub default_workspace_root: Option<String>,
    pub latex_toolchain_path: Option<String>,
}

fn read_optional_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        env::var(key)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            default_workspace_root: read_optional_env(&[
                "DEFAULT_WORKSPACE_ROOT",
                "VITE_DEFAULT_WORKSPACE_ROOT",
            ]),
            latex_toolchain_path: read_optional_env(&[
                "LATEX_TOOLCHAIN_PATH",
                "VITE_LATEX_TOOLCHAIN_PATH",
            ]),
        }
    }
}
