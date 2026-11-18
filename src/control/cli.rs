use super::{
    ApiError, ApiResult, Command, Descriptor, Operation, Request, Response, SCHEMA_VERSION, Shell,
};
use crate::model::{Agent, SessionColor};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

mod native;

pub const HELP: &str = r#"AI Terminal native management CLI

Usage: ai-terminal-cli [--data-dir PATH] [--json] COMMAND

  instance info                           Manager identity and caller session
  workspace show                          Navigation metadata and live status
  workspace apply --file PATH|-            Validate and apply a team plan
      [--dry-run] [--if-revision STAMP]
  session list [--offset N] [--limit N]     Paginated list (default 100, max 200)
  session show ID|current
  session create --name NAME --directory PATH [--shell powershell|cmd|bash]
      [--color COLOR] [--group ID] [--before ID|--after ID]
  session rename ID --name NAME
  session color ID --color none|red|green|amber|blue|pink|purple
  session group ID --group ID|none
  session move ID --before ID|--after ID
  session start ID [--activate]            Start its Shell, optionally switching
  session activate ID                      Open and switch to its Shell
  session run ID --agent codex|claude|qoder|qodercn|kimi|zcode|trae [--activate]
  session close ID [--force]               Running sessions require --force
  group list
  group create --name NAME --members ID,ID,...
  group rename ID --name NAME
  group dissolve ID                        Keep all member sessions
  request show REQUEST_ID                  Recover a recent command receipt

IDs are full UUIDs; current refers to the caller's managed session, never selection.
Commands emit UTF-8 JSON by default (--json is accepted for Agent compatibility).
--request-id UUID reuses an in-memory receipt; never blindly retry timed-out mutations.
No running manager: fail without editing sessions.json or starting another manager.
Run starts an Agent only in a session with no running terminal. It sends no prompt,
accepts no trust dialog, and leaves authentication/permissions to the installed CLI.
See docs/cli.md and docs/skills.md for the protocol, limits and team examples.
"#;

#[derive(Default)]
struct Args {
    words: Vec<String>,
    flags: BTreeMap<String, String>,
}
impl Args {
    fn parse(arguments: Vec<OsString>) -> ApiResult<Self> {
        let mut result = Self::default();
        let mut it = arguments.into_iter();
        while let Some(arg) = it.next() {
            let arg = arg
                .into_string()
                .map_err(|_| invalid("Arguments must be Unicode"))?;
            if let Some(key) = arg.strip_prefix("--") {
                let value = if matches!(key, "json" | "dry-run" | "force" | "activate" | "submit") {
                    "true".into()
                } else {
                    it.next()
                        .ok_or_else(|| invalid(format!("Missing value for --{key}")))?
                        .into_string()
                        .map_err(|_| invalid("Arguments must be Unicode"))?
                };
                if result.flags.insert(key.into(), value).is_some() {
                    return Err(invalid(format!("Duplicate --{key}")));
                }
            } else {
                result.words.push(arg);
            }
        }
        Ok(result)
    }
    fn take(&mut self, key: &str) -> Option<String> {
        self.flags.remove(key)
    }
    fn required(&mut self, key: &str) -> ApiResult<String> {
        self.take(key)
            .ok_or_else(|| invalid(format!("Missing --{key}")))
    }
    fn yes(&mut self, key: &str) -> bool {
        self.take(key).is_some()
    }
    fn id(&self) -> ApiResult<String> {
        self.words
            .get(2)
            .cloned()
            .ok_or_else(|| invalid("Missing target ID"))
    }
    fn finish(&self, words: usize) -> ApiResult<()> {
        if !self.flags.is_empty() || self.words.len() != words {
            return Err(invalid(format!(
                "Unexpected arguments; use --help. Unknown options: {}",
                self.flags.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(())
    }
}
fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new("invalid_argument", message)
}
fn number(value: Option<String>, default: usize) -> ApiResult<usize> {
    value
        .map(|v| {
            v.parse()
                .map_err(|_| invalid("Expected a nonnegative integer"))
        })
        .unwrap_or(Ok(default))
}
fn color(value: &str) -> ApiResult<Option<SessionColor>> {
    match value {
        "none" => Ok(None),
        "red" => Ok(Some(SessionColor::Red)),
        "green" => Ok(Some(SessionColor::Green)),
        "amber" | "yellow" => Ok(Some(SessionColor::Amber)),
        "blue" => Ok(Some(SessionColor::Blue)),
        "pink" => Ok(Some(SessionColor::Pink)),
        "purple" => Ok(Some(SessionColor::Purple)),
        _ => Err(invalid(
            "Unknown color; use none, red, green, amber, blue, pink, purple",
        )),
    }
}
pub fn agent(value: &str) -> ApiResult<Agent> {
    match value {
        "codex" => Ok(Agent::Codex),
        "claude" => Ok(Agent::Claude),
        "qoder" | "qodercli" => Ok(Agent::Qoder),
        "qodercn" | "qoderclicn" => Ok(Agent::QoderCn),
        "kimi" => Ok(Agent::Kimi),
        "zcode" => Ok(Agent::ZCode),
        "trae" | "traecli" => Ok(Agent::Trae),
        _ => Err(invalid("Unknown Agent")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema_version: u32,
    operations: Vec<Operation>,
    #[serde(default)]
    if_revision: Option<String>,
}
fn plan(path: &str) -> ApiResult<Plan> {
    let mut bytes = Vec::new();
    let base = if path == "-" {
        std::io::stdin()
            .take(super::MAX_FRAME as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| invalid(e.to_string()))?;
        std::env::current_dir().map_err(|e| invalid(e.to_string()))?
    } else {
        let absolute = std::path::absolute(path).map_err(|e| invalid(e.to_string()))?;
        std::fs::File::open(&absolute)
            .map_err(|e| invalid(e.to_string()))?
            .take(super::MAX_FRAME as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| invalid(e.to_string()))?;
        absolute.parent().unwrap_or(Path::new(".")).to_owned()
    };
    if bytes.len() > super::MAX_FRAME {
        return Err(invalid("Plan exceeds 256 KiB"));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    let mut plan: Plan = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    if plan.schema_version != SCHEMA_VERSION {
        return Err(invalid("Unsupported plan schema_version"));
    }
    for op in &mut plan.operations {
        if let Operation::Create { directory, .. } = op {
            *directory =
                std::path::absolute(base.join(&directory)).map_err(|e| invalid(e.to_string()))?;
        }
    }
    Ok(plan)
}

pub struct Parsed {
    pub directory: Option<PathBuf>,
    pub request_id: String,
    pub command: Command,
}
pub fn parse(arguments: Vec<OsString>) -> ApiResult<Parsed> {
    let mut args = Args::parse(arguments)?;
    args.take("json");
    let directory = args.take("data-dir").map(PathBuf::from);
    let request_id = args
        .take("request-id")
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(invalid("--request-id must be a UUID"));
    }
    if let Some((action, count)) = native::parse(&mut args)? {
        args.finish(count)?;
        return Ok(Parsed {
            directory,
            request_id,
            command: Command::Native { action },
        });
    }
    let action = (
        args.words.first().cloned().unwrap_or_default(),
        args.words.get(1).cloned().unwrap_or_default(),
    );
    let mut operations = Vec::new();
    let mut command = None;
    let mut count = 2;
    match (action.0.as_str(), action.1.as_str()) {
        ("instance", "info" | "capabilities") => command = Some(Command::Info),
        ("workspace", "show") => command = Some(Command::WorkspaceShow),
        ("workspace", "apply") => {
            let p = plan(&args.required("file")?)?;
            let if_revision = args.take("if-revision").or(p.if_revision);
            command = Some(Command::Apply {
                operations: p.operations,
                dry_run: args.yes("dry-run"),
                if_revision,
            });
        }
        ("session", "list") => {
            command = Some(Command::SessionList {
                offset: number(args.take("offset"), 0)?,
                limit: number(args.take("limit"), 100)?,
            })
        }
        ("session", "show") => {
            count = 3;
            command = Some(Command::SessionShow { id: args.id()? });
        }
        ("session", "create") => {
            let name = args.required("name")?;
            let directory = std::path::absolute(args.required("directory")?)
                .map_err(|e| invalid(e.to_string()))?;
            let shell = match args.take("shell").as_deref().unwrap_or("powershell") {
                "powershell" => Shell::Powershell,
                "cmd" => Shell::Cmd,
                "bash" => Shell::Bash,
                _ => return Err(invalid("Unknown shell")),
            };
            let color = args.take("color").map(|v| color(&v)).transpose()?.flatten();
            operations.push(Operation::Create {
                key: Some("created".into()),
                name,
                directory,
                shell,
                color,
            });
            if let Some(group) = args.take("group") {
                operations.push(Operation::Assign {
                    id: "$created".into(),
                    group: Some(group),
                });
            }
            let before = args.take("before");
            let after = args.take("after");
            if before.is_some() || after.is_some() {
                operations.push(Operation::Move {
                    id: "$created".into(),
                    before,
                    after,
                });
            }
        }
        ("session", "rename") => {
            count = 3;
            operations.push(Operation::Rename {
                id: args.id()?,
                name: args.required("name")?,
            });
        }
        ("session", "color") => {
            count = 3;
            operations.push(Operation::Color {
                id: args.id()?,
                color: color(&args.required("color")?)?,
            });
        }
        ("session", "group") => {
            count = 3;
            let group = args.required("group")?;
            operations.push(Operation::Assign {
                id: args.id()?,
                group: (group != "none").then_some(group),
            });
        }
        ("session", "move") => {
            count = 3;
            operations.push(Operation::Move {
                id: args.id()?,
                before: args.take("before"),
                after: args.take("after"),
            });
        }
        ("session", "start" | "activate") => {
            count = 3;
            command = Some(Command::Start {
                id: args.id()?,
                activate: args.yes("activate") || action.1 == "activate",
            });
        }
        ("session", "run") => {
            count = 3;
            command = Some(Command::Run {
                id: args.id()?,
                agent: agent(&args.required("agent")?)?,
                activate: args.yes("activate"),
            });
        }
        ("session", "close") => {
            count = 3;
            command = Some(Command::Close {
                id: args.id()?,
                force: args.yes("force"),
            });
        }
        ("group", "list") => command = Some(Command::GroupList),
        ("group", "create") => operations.push(Operation::GroupCreate {
            key: None,
            name: args.required("name")?,
            members: args
                .required("members")?
                .split(',')
                .map(str::to_owned)
                .collect(),
        }),
        ("group", "rename") => {
            count = 3;
            operations.push(Operation::GroupRename {
                id: args.id()?,
                name: args.required("name")?,
            });
        }
        ("group", "dissolve") => {
            count = 3;
            operations.push(Operation::GroupDissolve { id: args.id()? });
        }
        ("request", "show") => {
            count = 3;
            command = Some(Command::Receipt { id: args.id()? });
        }
        _ => return Err(invalid("Unknown command; use --help")),
    }
    args.finish(count)?;
    let command = command.unwrap_or(Command::Apply {
        operations,
        dry_run: false,
        if_revision: None,
    });
    Ok(Parsed {
        directory,
        request_id,
        command,
    })
}

fn target(directory: Option<&Path>) -> ApiResult<(Descriptor, Option<String>)> {
    let inherited = std::env::var("AI_TERMINAL_CONTROL_INSTANCE").ok();
    let descriptor = if let Some(directory) = directory {
        super::read_descriptor(directory)?
    } else if let Some(instance_id) = &inherited {
        Descriptor {
            schema_version: SCHEMA_VERSION,
            instance_id: instance_id.clone(),
            pid: 0,
            version: String::new(),
            data_directory: PathBuf::new(),
            endpoint: std::env::var("AI_TERMINAL_CONTROL_ENDPOINT")
                .map_err(|_| invalid("Incomplete inherited control capability"))?,
            token: std::env::var("AI_TERMINAL_CONTROL_TOKEN")
                .map_err(|_| invalid("Incomplete inherited control capability"))?,
        }
    } else {
        super::read_descriptor(&crate::model::data_directory())?
    };
    let caller = (inherited.as_deref() == Some(&descriptor.instance_id))
        .then(|| std::env::var("AI_TERMINAL_SESSION").ok())
        .flatten();
    Ok((descriptor, caller))
}
pub fn run(arguments: Vec<OsString>) -> i32 {
    if arguments.is_empty() || arguments.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}\n{}", native::HELP);
        return 0;
    }
    if arguments.len() == 1 && arguments[0] == "--version" {
        println!(
            "ai-terminal-cli {} (schema {})",
            env!("CARGO_PKG_VERSION"),
            SCHEMA_VERSION
        );
        return 0;
    }
    let mut request_id = String::new();
    let mut instance_id = String::new();
    let result: ApiResult<Response> = (|| {
        let parsed = parse(arguments)?;
        request_id = parsed.request_id.clone();
        let (descriptor, caller_session) = target(parsed.directory.as_deref())?;
        instance_id = descriptor.instance_id.clone();
        if matches!(parsed.command, Command::Native { .. }) {
            let check = super::call(
                &descriptor,
                Request {
                    schema_version: SCHEMA_VERSION,
                    request_id: uuid::Uuid::new_v4().to_string(),
                    caller_session: caller_session.clone(),
                    action: Command::Info,
                },
            )?;
            if check
                .result
                .as_ref()
                .and_then(|v| v["capabilities"]["native_controls"].as_bool())
                != Some(true)
            {
                return Err(ApiError::new(
                    "unsupported_command",
                    "This manager predates native controls. Open the newer preview; existing sessions are left running.",
                ));
            }
        }
        let request = Request {
            schema_version: SCHEMA_VERSION,
            request_id: parsed.request_id,
            caller_session: caller_session.clone(),
            action: parsed.command,
        };
        let response = super::call(&descriptor, request)?;
        Ok(response)
    })();
    let response = result.unwrap_or_else(|e| Response::new(&request_id, &instance_id, Err(e)));
    println!(
        "{}",
        serde_json::to_string(&response).expect("serializable response")
    );
    if response.ok {
        0
    } else {
        match response
            .error
            .as_ref()
            .map(|e| e.code.as_str())
            .unwrap_or("")
        {
            "invalid_argument"
            | "invalid_directory"
            | "too_large"
            | "unsupported_version"
            | "unsupported_command" => 2,
            "not_running" => 3,
            "not_found" | "no_current_session" => 4,
            "conflict" | "session_running" | "request_id_reused" => 5,
            "busy" => 6,
            _ => 1,
        }
    }
}
