use std::path::PathBuf;

const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Debug, Clone)]
pub struct BackendSettings {
    pub shell: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            shell: DEFAULT_SHELL.to_string(),
            args: vec![],
            env: Default::default(),
            working_directory: None,
        }
    }
}
