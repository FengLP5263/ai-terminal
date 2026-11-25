//! Read conversation titles from the Agents' public metadata interfaces.
use ai_terminal::{discovery::CliSearch, model::Agent};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, Instant},
};

const QUEUE_SIZE: usize = 8;
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PAGES: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Target {
    pub local_id: String,
    pub generation: String,
    pub invocation: Option<String>,
    pub agent: Agent,
    pub conversation_id: String,
}

type Request = (Target, PathBuf);
type Title = (Target, String);

pub(super) struct Lookup {
    ctx: egui::Context,
    requests: Option<SyncSender<Request>>,
    results: Receiver<Title>,
    reply: SyncSender<Title>,
}

impl Lookup {
    pub fn new(ctx: egui::Context) -> Self {
        let (reply, results) = mpsc::sync_channel(QUEUE_SIZE);
        Self {
            ctx,
            requests: None,
            results,
            reply,
        }
    }

    pub fn request(&mut self, target: Target, directory: PathBuf) -> bool {
        if self.requests.is_none() {
            let (requests, receiver) = mpsc::sync_channel::<Request>(QUEUE_SIZE);
            let reply = self.reply.clone();
            let ctx = self.ctx.clone();
            let worker = std::thread::Builder::new()
                .name("agent-session-titles".into())
                .spawn(move || {
                    for (target, directory) in receiver {
                        if let Some(title) = lookup_title(&target, &directory)
                            && reply.try_send((target, title)).is_ok()
                        {
                            ctx.request_repaint_of(egui::ViewportId::ROOT);
                        }
                    }
                });
            if worker.is_err() {
                return false;
            }
            self.requests = Some(requests);
        }
        self.requests
            .as_ref()
            .is_some_and(|requests| requests.try_send((target, directory)).is_ok())
    }

    pub fn titles(&self) -> impl Iterator<Item = Title> + '_ {
        self.results.try_iter()
    }
}

fn lookup_title(target: &Target, directory: &Path) -> Option<String> {
    let search = CliSearch::current();
    let program = search.find(target.agent)?;
    let mut command = Command::new(program);
    let arguments: &[&str] = match target.agent {
        Agent::Qoder | Agent::QoderCn => &["--acp"],
        Agent::Trae => &["app-server", "--stdio"],
        Agent::ZCode => &["app-server"],
        _ => return None,
    };
    command
        .args(arguments)
        .current_dir(directory)
        .env("PATH", search.child_path().ok()?)
        .env_remove("AI_TERMINAL_ENDPOINT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().ok()?;
    let title = match crate::platform::ProcessJob::attach(child.id()) {
        Ok(job) => {
            let title = (|| {
                let stdin = child.stdin.take()?;
                let stdout = child.stdout.take()?;
                let (sender, responses) = mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .name("agent-title-output".into())
                    .spawn(move || read_responses(stdout, sender))
                    .ok()?;
                query_title(
                    stdin,
                    &responses,
                    directory,
                    target.agent,
                    &target.conversation_id,
                )
            })();
            drop(job);
            title
        }
        Err(_) => None,
    };
    let _ = child.kill();
    let _ = child.wait();
    title
}

fn read_responses(stdout: impl Read, responses: SyncSender<Value>) {
    for line in BufReader::new(stdout.take(MAX_RESPONSE_BYTES)).lines() {
        let Ok(line) = line else { break };
        if let Ok(message) = serde_json::from_str::<Value>(&line)
            && (message.get("jsonrpc").is_none() || message["jsonrpc"] == "2.0")
            && message.get("method").is_none()
            && message["id"].is_u64()
            && responses.send(message).is_err()
        {
            break;
        }
    }
}

fn query_title(
    stdin: impl Write,
    responses: &Receiver<Value>,
    directory: &Path,
    agent: Agent,
    conversation_id: &str,
) -> Option<String> {
    let mut rpc = Rpc {
        stdin,
        responses,
        deadline: Instant::now() + LOOKUP_TIMEOUT,
        jsonrpc: matches!(agent, Agent::Qoder | Agent::QoderCn),
    };
    let title = match agent {
        Agent::Qoder | Agent::QoderCn => qoder_title(&mut rpc, directory, conversation_id),
        Agent::Trae => trae_title(&mut rpc, conversation_id),
        Agent::ZCode => zcode_title(&mut rpc, conversation_id),
        _ => None,
    }?;
    (title.len() <= 512).then_some(title)
}

struct Rpc<'a, W> {
    stdin: W,
    responses: &'a Receiver<Value>,
    deadline: Instant,
    jsonrpc: bool,
}

impl<W: Write> Rpc<'_, W> {
    fn send(&mut self, message: Value) -> Option<()> {
        writeln!(self.stdin, "{message}").ok()?;
        self.stdin.flush().ok()
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Option<Value> {
        let mut message = json!({"id": id, "method": method, "params": params});
        if self.jsonrpc {
            message["jsonrpc"] = json!("2.0");
        }
        self.send(message)?;
        loop {
            let remaining = self.deadline.checked_duration_since(Instant::now())?;
            let mut response = self.responses.recv_timeout(remaining).ok()?;
            if response["id"].as_u64() == Some(id) {
                return response.as_object_mut()?.remove("result");
            }
        }
    }
}

fn qoder_title(
    rpc: &mut Rpc<'_, impl Write>,
    directory: &Path,
    conversation_id: &str,
) -> Option<String> {
    let init = rpc.request(
        1,
        "initialize",
        json!({"protocolVersion": 1, "clientCapabilities": {}}),
    )?;
    if init["protocolVersion"] != 1
        || !init["agentCapabilities"]["sessionCapabilities"]["list"].is_object()
    {
        return None;
    }

    let mut params = json!({"cwd": directory.to_str()?});
    for page in 0..MAX_PAGES {
        let result = rpc.request(u64::from(page) + 2, "session/list", params.clone())?;
        if let Some(session) = result["sessions"]
            .as_array()?
            .iter()
            .find(|row| row["sessionId"].as_str() == Some(conversation_id))
        {
            return session["title"].as_str().map(str::to_owned);
        }
        let cursor = result["nextCursor"].as_str().filter(|s| s.len() <= 4096)?;
        params["cursor"] = json!(cursor);
    }
    None
}

fn trae_title(rpc: &mut Rpc<'_, impl Write>, conversation_id: &str) -> Option<String> {
    rpc.request(
        1,
        "initialize",
        json!({"clientInfo": {"name": "ai-terminal", "version": env!("CARGO_PKG_VERSION")}}),
    )?;
    rpc.send(json!({"method": "initialized"}))?;
    let result = rpc.request(
        2,
        "thread/read",
        json!({"threadId": conversation_id, "includeTurns": false}),
    )?;
    let thread = &result["thread"];
    if thread["id"].as_str() != Some(conversation_id) {
        return None;
    }
    thread["name"].as_str().map(str::to_owned)
}

fn zcode_title(rpc: &mut Rpc<'_, impl Write>, conversation_id: &str) -> Option<String> {
    let result = rpc.request(1, "session/list", json!({"sessionIds": [conversation_id]}))?;
    result["sessions"]
        .as_array()?
        .iter()
        .find(|row| row["sessionId"].as_str() == Some(conversation_id))?["title"]
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_uses_the_exact_conversation_and_follows_pagination() {
        let (tx, rx) = mpsc::channel();
        tx.send(json!({"id": 1, "result": {"protocolVersion": 1,
            "agentCapabilities": {"sessionCapabilities": {"list": {}}}}}))
            .unwrap();
        tx.send(json!({"id": 2, "result": {"sessions": [
            {"sessionId":"newer", "title":"Another conversation"}
        ], "nextCursor":"page-2"}}))
            .unwrap();
        tx.send(json!({"id": 3, "result": {"sessions": [
            {"sessionId":"chosen", "title":"Qoder 会话 🧪"}
        ]}}))
        .unwrap();
        let mut sent = Vec::new();
        assert_eq!(
            query_title(&mut sent, &rx, Path::new("project"), Agent::Qoder, "chosen").as_deref(),
            Some("Qoder 会话 🧪")
        );
        let sent = String::from_utf8(sent).unwrap();
        let requests: Vec<Value> = sent
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(requests[0]["method"], "initialize");
        assert_eq!(requests[1]["method"], "session/list");
        assert_eq!(requests[1]["params"]["cwd"], "project");
        assert_eq!(requests[2]["params"]["cursor"], "page-2");
    }

    #[test]
    fn trae_uses_exact_thread_name_and_never_requests_turns() {
        let (tx, rx) = mpsc::channel();
        tx.send(json!({"id": 1, "result": {}})).unwrap();
        tx.send(json!({"id": 2, "result": {"thread": {"id": "chosen", "name": "TRAE title", "preview": "ignored"}}})).unwrap();
        let mut sent = Vec::new();
        assert_eq!(
            query_title(&mut sent, &rx, Path::new("project"), Agent::Trae, "chosen").as_deref(),
            Some("TRAE title")
        );
        let sent = String::from_utf8(sent).unwrap();
        let requests: Vec<Value> = sent
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(requests[1]["method"], "initialized");
        assert_eq!(requests[2]["method"], "thread/read");
        assert_eq!(
            requests[2]["params"],
            json!({"threadId":"chosen","includeTurns":false})
        );
    }

    #[test]
    fn zcode_queries_only_the_requested_session_metadata() {
        let (tx, rx) = mpsc::channel();
        tx.send(json!({"id": 1, "result": {"sessions": [
            {"sessionId":"another", "title":"Ignore"},
            {"sessionId":"chosen", "title":"ZCode title"}
        ]}}))
        .unwrap();
        let mut sent = Vec::new();
        assert_eq!(
            query_title(&mut sent, &rx, Path::new("project"), Agent::ZCode, "chosen").as_deref(),
            Some("ZCode title")
        );
        let request: Value = serde_json::from_slice(&sent).unwrap();
        assert_eq!(request["method"], "session/list");
        assert_eq!(request["params"], json!({"sessionIds":["chosen"]}));
        assert!(request.get("jsonrpc").is_none());
    }
}
