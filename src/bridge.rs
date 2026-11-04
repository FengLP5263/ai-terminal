use crate::status::HookEvent;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

const MAX_FRAME: u64 = 8192;
#[derive(Serialize, Deserialize)]
struct Envelope {
    token: String,
    event: HookEvent,
}

pub struct Bridge {
    pub endpoint: String,
    pub token: String,
    pub receiver: Receiver<HookEvent>,
    running: Arc<AtomicBool>,
}
impl Bridge {
    pub fn start(wake: Arc<dyn Fn() + Send + Sync>) -> std::io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let endpoint = listener.local_addr()?.to_string();
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let secret = token.clone();
        let running = Arc::new(AtomicBool::new(true));
        let alive = running.clone();
        let (sender, receiver) = mpsc::sync_channel(256);
        std::thread::Builder::new()
            .name("agent-events".into())
            .spawn(move || {
                for connection in listener.incoming() {
                    if !alive.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(mut stream) = connection else {
                        continue;
                    };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(180)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
                    let mut line = String::new();
                    if BufReader::new((&stream).take(MAX_FRAME + 1))
                        .read_line(&mut line)
                        .is_err()
                        || line.len() as u64 > MAX_FRAME
                    {
                        continue;
                    }
                    let Ok(envelope) = serde_json::from_str::<Envelope>(&line) else {
                        continue;
                    };
                    if envelope.token != secret {
                        continue;
                    }
                    match sender.try_send(envelope.event) {
                        Ok(()) => {
                            // Queue acceptance must not wait for a UI lock or a long first paint.
                            let _ = stream.write_all(b"OK\n");
                            wake();
                        }
                        Err(mpsc::TrySendError::Full(_)) => {
                            let _ = stream.write_all(b"BSY");
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => break,
                    }
                }
            })?;
        Ok(Self {
            endpoint,
            token,
            receiver,
            running,
        })
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = TcpStream::connect(&self.endpoint);
    }
}

pub fn send(endpoint: &str, token: &str, event: HookEvent) -> std::io::Result<()> {
    let address: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|_| std::io::Error::other("invalid bridge"))?;
    if !address.ip().is_loopback() {
        return Err(std::io::Error::other("bridge must be local"));
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(150))?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(150)))?;
    let mut frame = serde_json::to_vec(&Envelope {
        token: token.into(),
        event,
    })?;
    if frame.len() as u64 >= MAX_FRAME {
        return Err(std::io::Error::other("event too large"));
    }
    frame.push(b'\n');
    stream.write_all(&frame)?;
    let mut ack = [0; 3];
    stream.read_exact(&mut ack)?;
    if ack == *b"BSY" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "event queue busy",
        ));
    }
    if ack != *b"OK\n" {
        return Err(std::io::Error::other("no acknowledgement"));
    }
    Ok(())
}

/// Runs before GUI initialization. Does not load fonts, GPU, user configs or API keys.
pub fn report_hook() {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = std::env::var("AI_TERMINAL_ENDPOINT")?;
        let token = std::env::var("AI_TERMINAL_TOKEN")?;
        let local_id = std::env::var("AI_TERMINAL_SESSION")?;
        let generation = std::env::var("AI_TERMINAL_GENERATION")?;
        let mut input = String::new();
        std::io::stdin()
            .take(1_048_577)
            .read_to_string(&mut input)?;
        if input.len() > 1_048_576 {
            return Ok(());
        }
        let value: serde_json::Value = serde_json::from_str(&input)?;
        let field = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(200).collect::<String>())
        };
        if ["agent_id", "agentId"]
            .iter()
            .any(|key| field(key).is_some_and(|id| !id.is_empty()))
        {
            return Ok(());
        }
        let event = HookEvent {
            invocation: std::env::var("AI_TERMINAL_INVOCATION").ok(),
            process_id: None,
            parent_invocation: None,
            console_processes: None,
            event_id: uuid::Uuid::new_v4().to_string(),
            local_id,
            generation,
            name: field("hook_event_name").unwrap_or_default(),
            session_id: field("session_id").unwrap_or_default(),
            turn_id: field("turn_id").or_else(|| field("turnId")),
            tool_name: field("tool_name"),
            tool_use_id: field("tool_use_id"),
            notification_type: field("notification_type"),
            source: field("source"),
            exit_code: value
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok()),
            background_running: value
                .get("background_tasks")
                .and_then(|v| v.as_array())
                .is_some_and(|tasks| {
                    tasks
                        .iter()
                        .any(|t| t.get("status").and_then(|v| v.as_str()) == Some("running"))
                }),
        };
        for attempt in 0..3 {
            if send(&endpoint, &token, event.clone()).is_ok() {
                return Ok(());
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        Ok(())
    })();
    // Advisory hook: never deny a tool, emit context, or block a user's agent.
    let _ = result;
    println!("{{}}");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> HookEvent {
        HookEvent {
            invocation: None,
            process_id: None,
            parent_invocation: None,
            console_processes: None,
            event_id: "id".into(),
            local_id: "one".into(),
            generation: "launch".into(),
            name: "Stop".into(),
            session_id: "agent".into(),
            turn_id: None,
            tool_name: None,
            tool_use_id: None,
            notification_type: None,
            source: None,
            background_running: false,
            exit_code: None,
        }
    }
    #[test]
    fn authenticated_delivery_wakes_background_ui_and_acknowledges() {
        let (wake_sender, wake_receiver) = mpsc::channel();
        let bridge = Bridge::start(Arc::new(move || {
            let _ = wake_sender.send(());
        }))
        .unwrap();
        send(&bridge.endpoint, &bridge.token, fixture()).unwrap();
        assert_eq!(
            bridge
                .receiver
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .name,
            "Stop"
        );
        wake_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("accepted event must wake the UI");
    }
    #[test]
    fn wrong_token_cannot_change_session_status() {
        let bridge = Bridge::start(Arc::new(|| {})).unwrap();
        assert!(send(&bridge.endpoint, "wrong", fixture()).is_err());
        assert!(bridge.receiver.try_recv().is_err());
        send(&bridge.endpoint, &bridge.token, fixture()).unwrap();
    }
    #[test]
    fn full_queue_explicitly_rejects_and_allows_redelivery_after_drain() {
        let bridge = Bridge::start(Arc::new(|| {})).unwrap();
        for _ in 0..256 {
            send(&bridge.endpoint, &bridge.token, fixture()).unwrap();
        }
        assert_eq!(
            send(&bridge.endpoint, &bridge.token, fixture())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
        bridge.receiver.recv().unwrap();
        send(&bridge.endpoint, &bridge.token, fixture()).unwrap();
    }
}
