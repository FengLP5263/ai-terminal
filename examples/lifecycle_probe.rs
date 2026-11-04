//! Bounded real CLI check. Startup mode sends no prompt; offline mode uses only a loopback fixture.
//! Never approves hook trust or submits a real-provider inference prompt.
use ai_terminal::{
    bridge::Bridge,
    launch,
    model::{Agent, SessionSpec},
};
use egui_term::{BackendCommand, BackendSettings, TerminalBackend};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let agent = args.get(1).expect("codex or claude");
    assert!(agent == "codex" || agent == "claude");
    let output = PathBuf::from(args.get(2).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let bridge = Bridge::start(Arc::new(|| {})).unwrap();
    let spec = SessionSpec {
        id: "startup-probe".into(),
        name: "probe".into(),
        directory: std::env::var_os("AI_TERMINAL_PROBE_CWD")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap()),
        agent: Agent::Shell,
        agent_session_id: None,
        color: None,
    };
    let helper = std::env::var_os("AI_TERMINAL_PROBE_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("ai-terminal-hook.exe")
        });
    let offline = args.get(3).is_some_and(|s| s == "offline");
    let fixture = offline.then(offline_fixture::Fixture::start);
    let mut plan = launch::build(&spec, &bridge, &output.join("runtime"), &helper).unwrap();
    if agent == "codex" {
        let log_dir = std::path::absolute(output.join("logs")).unwrap();
        std::fs::create_dir_all(&log_dir).unwrap();
        plan.environment.insert(
            "RUST_LOG".into(),
            "codex_hooks=debug,codex_core=info,codex_tui=info".into(),
        );
        plan.environment.insert(
            "AI_TERMINAL_PROBE_LOG_DIR".into(),
            log_dir.to_string_lossy().into_owned(),
        );
    }
    if let Some(fixture) = &fixture {
        plan.environment.insert(
            "AI_TERMINAL_PROBE_PROVIDER".into(),
            fixture.base_url.clone(),
        );
    }
    let (tx, rx) = mpsc::channel();
    let mut terminal = TerminalBackend::new(
        991,
        egui::Context::default(),
        tx,
        BackendSettings {
            shell: plan.executable,
            args: plan.arguments,
            env: plan.environment,
            working_directory: Some(spec.directory),
        },
    )
    .unwrap();
    let job = platform::ProcessJob::attach(terminal.pty_id()).unwrap();
    let started = Instant::now();
    let mut sent = false;
    let mut events = Vec::new();
    let mut ready = false;
    let mut inspected = false;
    let mut completed_command = false;
    let mut initial_text = String::new();
    let mut quit_stage = 0;
    while started.elapsed() < Duration::from_secs(30) {
        let text = terminal.plain_text();
        if !sent && text.contains("PS ") {
            terminal.process_command(BackendCommand::Write(
                format!(
                    "{agent}{}\r",
                    if offline {
                        " --no-daemon -c check_for_update_on_startup=false -c log_dir=\"$env:AI_TERMINAL_PROBE_LOG_DIR\" -c model_provider='ai_terminal_fixture' -c model_providers.ai_terminal_fixture.name='OfflineFixture' -c model_providers.ai_terminal_fixture.base_url=\"$env:AI_TERMINAL_PROBE_PROVIDER\" -c model_providers.ai_terminal_fixture.wire_api='responses' -c model_providers.ai_terminal_fixture.requires_openai_auth=false -c model_providers.ai_terminal_fixture.supports_websockets=false -c model='gpt-5.4'"
                    } else if agent == "codex" {
                        " --no-daemon -c check_for_update_on_startup=false -c log_dir=\"$env:AI_TERMINAL_PROBE_LOG_DIR\""
                    } else {
                        ""
                    }
                )
                .into_bytes(),
            ));
            sent = true;
        }
        for e in bridge.receiver.try_iter() {
            if events.len() < 128 {
                events.push(serde_json::json!({"name":e.name,"session_id":e.session_id,"invocation":e.invocation,"source":e.source}));
            }
        }
        ready |= if agent == "codex" {
            text.contains("Ask Codex")
        } else {
            text.contains("Claude Code") && text.contains('❯')
        };
        if ready
            && agent == "codex"
            && !inspected
            && started.elapsed() > Duration::from_secs(8)
            && !args.get(3).is_some_and(|s| s == "exit")
            && !offline
        {
            let command = if args.get(3).is_some_and(|s| s == "warnings") {
                b"\x1bOQ".as_slice()
            } else {
                b"/hooks".as_slice()
            };
            terminal.process_command(BackendCommand::Write(command.to_vec()));
            inspected = true;
            initial_text = text.clone();
        }
        if inspected
            && !completed_command
            && started.elapsed() > Duration::from_secs(11)
            && (text.contains("view and manage lifecycle hooks")
                || text.lines().any(|line| line.trim() == "› /hooks"))
            && !args.get(3).is_some_and(|s| s == "warnings")
            && !text.contains("Trust all")
            && !text.contains("Review hooks")
        {
            terminal.process_command(BackendCommand::Write(b"\r".to_vec()));
            completed_command = true;
        }
        if offline && ready && quit_stage == 0 && started.elapsed() > Duration::from_secs(8) {
            terminal.process_command(BackendCommand::Write(b"Reply with fixture-ready.".to_vec()));
            quit_stage = 1;
        }
        if offline && quit_stage == 1 && started.elapsed() > Duration::from_secs(10) {
            terminal.process_command(BackendCommand::Write(vec![13]));
            quit_stage = 2;
        }
        if offline && events.iter().any(|e| e["name"] == "Stop") {
            break;
        }
        if args.get(3).is_some_and(|s| s == "exit")
            && ready
            && started.elapsed() > Duration::from_secs(8 + quit_stage * 3)
            && quit_stage < 2
        {
            terminal.process_command(BackendCommand::Write(vec![4]));
            quit_stage += 1;
        }
        if (ready && agent == "claude" && started.elapsed() > Duration::from_secs(12))
            || (inspected && started.elapsed() > Duration::from_secs(22))
        {
            break;
        }
        let _ = rx.recv_timeout(Duration::from_millis(50));
    }
    let text = terminal.plain_text();
    let result = serde_json::json!({"agent":agent,"startup_command_sent":sent,"ready":ready,"events":events,"model_request_sent":offline,"offline_fixture_requests":fixture.as_ref().map(|f|f.requests()),"pty_pid":terminal.pty_id(),"startup_text":text,"initial_text":initial_text,"terminal_mode":format!("{:?}",terminal.sync().terminal_mode)});
    std::fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        serde_json::json!({"agent":agent,"ready":ready,"events":events,"output":output})
    );
    drop(job);
    drop(terminal);
}

mod offline_fixture {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Duration,
    };
    pub struct Fixture {
        pub base_url: String,
        count: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
    }
    impl Fixture {
        pub fn requests(&self) -> usize {
            self.count.load(Ordering::Relaxed)
        }
        pub fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
            let count = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let n = count.clone();
            let done = stop.clone();
            std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    let Ok((mut stream, _)) = listener.accept() else {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    };
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                    let mut bytes = Vec::new();
                    let mut chunk = [0; 4096];
                    while bytes.len() < 1_048_576 {
                        match stream.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(len) => bytes.extend_from_slice(&chunk[..len]),
                        }
                        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header = String::from_utf8_lossy(&bytes[..end]);
                            let length = header
                                .lines()
                                .find_map(|l| {
                                    l.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|v| v.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if bytes.len() >= end + 4 + length {
                                break;
                            }
                        }
                    }
                    if bytes.is_empty() {
                        continue;
                    }
                    if bytes.starts_with(b"GET ") {
                        let _=stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        continue;
                    }
                    n.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "local fixture request: {}",
                        String::from_utf8_lossy(&bytes).lines().next().unwrap_or("")
                    );
                    // Never log request content, headers or credentials.
                    let item = serde_json::json!({"id":"msg_fixture","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"fixture-ready","annotations":[]}]});
                    let events = [
                        serde_json::json!({"type":"response.created","response":{"id":"resp_fixture","status":"in_progress","output":[]}}),
                        serde_json::json!({"type":"response.output_item.done","output_index":0,"item":item}),
                        serde_json::json!({"type":"response.completed","response":{"id":"resp_fixture","status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}),
                    ];
                    let body = events
                        .iter()
                        .map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap(), e))
                        .collect::<String>();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            });
            Self {
                base_url,
                count,
                stop,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }
}
