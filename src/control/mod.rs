//! Versioned local native API. Explicit terminal I/O is transient, never logged or persisted.
pub mod actions;
pub mod cli;
mod instance;
mod transport;
mod workspace;
pub use instance::ManagerLease;

use crate::model::{Agent, SessionColor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
pub use transport::{ControlServer, Descriptor, Pending, call, read_descriptor};
pub use workspace::{Prepared, prepare, resolve, revision};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_FRAME: usize = 262_144;
pub const MAX_RESPONSE: usize = 1_048_576;
pub const MAX_OPERATIONS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub schema_version: u32,
    pub request_id: String,
    #[serde(default)]
    pub caller_session: Option<String>,
    #[serde(flatten)]
    pub action: Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Native {
        #[serde(flatten)]
        action: actions::Action,
    },
    Info,
    Receipt {
        id: String,
    },
    SessionList {
        offset: usize,
        limit: usize,
    },
    SessionShow {
        id: String,
    },
    GroupList,
    WorkspaceShow,
    Apply {
        operations: Vec<Operation>,
        #[serde(default)]
        dry_run: bool,
        #[serde(default)]
        if_revision: Option<String>,
    },
    Start {
        id: String,
        activate: bool,
    },
    Run {
        id: String,
        agent: Agent,
        activate: bool,
    },
    Close {
        id: String,
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shell {
    #[default]
    Powershell,
    Cmd,
    Bash,
}
impl Shell {
    pub fn agent(self) -> Agent {
        match self {
            Self::Powershell => Agent::Shell,
            Self::Cmd => Agent::Cmd,
            Self::Bash => Agent::Bash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Operation {
    #[serde(rename = "session.create")]
    Create {
        #[serde(default)]
        key: Option<String>,
        name: String,
        directory: PathBuf,
        #[serde(default)]
        shell: Shell,
        #[serde(default)]
        color: Option<SessionColor>,
    },
    #[serde(rename = "session.rename")]
    Rename { id: String, name: String },
    #[serde(rename = "session.color")]
    Color {
        id: String,
        color: Option<SessionColor>,
    },
    #[serde(rename = "session.group")]
    Assign { id: String, group: Option<String> },
    #[serde(rename = "session.move")]
    Move {
        id: String,
        #[serde(default)]
        before: Option<String>,
        #[serde(default)]
        after: Option<String>,
    },
    #[serde(rename = "group.create")]
    GroupCreate {
        #[serde(default)]
        key: Option<String>,
        name: String,
        members: Vec<String>,
    },
    #[serde(rename = "group.rename")]
    GroupRename { id: String, name: String },
    #[serde(rename = "group.dissolve")]
    GroupDissolve { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub schema_version: u32,
    pub request_id: String,
    pub instance_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}
impl Response {
    pub fn new(id: &str, instance: &str, result: ApiResult<Value>) -> Self {
        let (ok, value, error) = match result {
            Ok(v) => (true, Some(v), None),
            Err(e) => (false, None, Some(e)),
        };
        Self {
            schema_version: SCHEMA_VERSION,
            request_id: id.into(),
            instance_id: instance.into(),
            ok,
            result: value,
            error,
        }
    }
}
