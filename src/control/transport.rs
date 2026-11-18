use super::{ApiError, ApiResult, MAX_FRAME, MAX_RESPONSE, Request, Response, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

// Not Debug: credentials must never appear in diagnostics or CLI output.
#[derive(Clone, Serialize, Deserialize)]
pub struct Descriptor {
    pub schema_version: u32,
    pub instance_id: String,
    pub pid: u32,
    pub version: String,
    pub data_directory: PathBuf,
    pub endpoint: String,
    #[serde(skip)]
    pub token: String,
}
#[derive(Serialize, Deserialize)]
struct StoredDescriptor {
    #[serde(flatten)]
    descriptor: Descriptor,
    protected_token: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    token: String,
    instance_id: String,
    request: Request,
}
pub struct Pending {
    pub request: Request,
    pub reply: Sender<Response>,
    pub expires: Instant,
}
pub struct ControlServer {
    pub descriptor: Descriptor,
    pub receiver: Receiver<Pending>,
    running: Arc<AtomicBool>,
    path: PathBuf,
    _lock: File,
}

fn io_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::new("unavailable", e.to_string())
}
pub fn read_descriptor(directory: &Path) -> ApiResult<Descriptor> {
    let file = File::open(directory.join("runtime/control.json")).map_err(|_| {
        ApiError::new(
            "not_running",
            "No CLI-enabled manager found for this data directory; start the new version first",
        )
    })?;
    let mut bytes = Vec::new();
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > 16_384 {
        return Err(io_error("Invalid control descriptor"));
    }
    let stored: StoredDescriptor =
        serde_json::from_slice(&bytes).map_err(|_| io_error("Invalid control descriptor"))?;
    let mut descriptor = stored.descriptor;
    let selected = directory.canonicalize().map_err(io_error)?;
    let actual = descriptor.data_directory.canonicalize().map_err(io_error)?;
    if !selected
        .as_os_str()
        .eq_ignore_ascii_case(actual.as_os_str())
    {
        return Err(ApiError::new(
            "wrong_workspace",
            "Discovery file belongs to a different data directory; open the intended manager instead",
        ));
    }
    descriptor.token =
        String::from_utf8(protect(&stored.protected_token, false).map_err(|_| {
            io_error("Cannot decrypt this manager capability for the current Windows user")
        })?)
        .map_err(|_| io_error("Invalid manager capability"))?;
    if descriptor.schema_version != SCHEMA_VERSION {
        return Err(ApiError::new(
            "unsupported_version",
            "Unsupported manager control schema",
        ));
    }
    local_address(&descriptor.endpoint)?;
    Ok(descriptor)
}
fn local_address(endpoint: &str) -> ApiResult<SocketAddr> {
    let address: SocketAddr = endpoint
        .parse()
        .map_err(|_| io_error("Invalid local endpoint"))?;
    if address.ip() != std::net::IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(io_error("Control endpoint must be 127.0.0.1"));
    }
    Ok(address)
}

pub fn call(descriptor: &Descriptor, request: Request) -> ApiResult<Response> {
    let address = local_address(&descriptor.endpoint)?;
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_millis(500)).map_err(|_| {
            ApiError::new(
                "not_running",
                "Manager is unavailable; the saved descriptor may be stale",
            )
        })?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(io_error)?;
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(io_error)?;
    let expected = request.request_id.clone();
    let mut bytes = serde_json::to_vec(&Envelope {
        token: descriptor.token.clone(),
        instance_id: descriptor.instance_id.clone(),
        request,
    })
    .map_err(io_error)?;
    if bytes.len() > MAX_FRAME {
        return Err(ApiError::new(
            "too_large",
            "Management request exceeds 256 KiB",
        ));
    }
    bytes.push(b'\n');
    stream.write_all(&bytes).map_err(io_error)?;
    let reply = read_frame(&mut stream, MAX_RESPONSE, Duration::from_secs(12)).map_err(|_| ApiError::new("outcome_unknown", "No complete response. Query the workspace before retrying; do not repeat a mutation with a new request ID"))?;
    let response: Response =
        serde_json::from_slice(&reply).map_err(|_| io_error("Invalid manager response"))?;
    if response.schema_version != SCHEMA_VERSION
        || response.request_id != expected
        || response.instance_id != descriptor.instance_id
    {
        return Err(io_error("Manager response identity mismatch"));
    }
    Ok(response)
}

fn read_frame(stream: &mut TcpStream, limit: usize, timeout: Duration) -> std::io::Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut result = Vec::new();
    let mut buf = [0; 4096];
    while Instant::now() < deadline {
        match stream.read(&mut buf) {
            Ok(0) => return Err(std::io::Error::other("Incomplete frame")),
            Ok(n) => {
                let end = buf[..n].iter().position(|b| *b == b'\n');
                let slice = &buf[..end.unwrap_or(n)];
                if result.len() + slice.len() > limit {
                    return Err(std::io::Error::other("Frame too large"));
                }
                result.extend_from_slice(slice);
                if end.is_some() {
                    return Ok(result);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "Frame deadline expired",
    ))
}

impl ControlServer {
    pub fn start(directory: &Path, wake: Arc<dyn Fn() + Send + Sync>) -> std::io::Result<Self> {
        let directory = std::path::absolute(directory)?;
        std::fs::create_dir_all(directory.join("runtime"))?;
        let mut lock_options = OpenOptions::new();
        lock_options
            .read(true)
            .write(true)
            .create(true)
            .truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            lock_options.share_mode(0);
        }
        let lock = lock_options.open(directory.join("runtime/control.lock"))?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let descriptor = Descriptor {
            schema_version: SCHEMA_VERSION,
            instance_id: uuid::Uuid::new_v4().to_string(),
            pid: std::process::id(),
            version: env!("CARGO_PKG_VERSION").into(),
            data_directory: directory.clone(),
            endpoint: listener.local_addr()?.to_string(),
            token: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        };
        // Persist only DPAPI ciphertext, scoped to the current Windows user.
        let mut file = tempfile::NamedTempFile::new_in(directory.join("runtime"))?;
        let stored = StoredDescriptor {
            descriptor: descriptor.clone(),
            protected_token: protect(descriptor.token.as_bytes(), true)?,
        };
        serde_json::to_writer(&mut file, &stored)?;
        file.flush()?;
        let path = directory.join("runtime/control.json");
        file.persist(&path)
            .map_err(|e| std::io::Error::other(format!("Publish CLI capability: {}", e.error)))?;
        let running = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = mpsc::sync_channel(32);
        let (connections, clients) = mpsc::sync_channel::<TcpStream>(16);
        let clients = Arc::new(Mutex::new(clients));
        // Own discovery/lease before spawning: partial thread-start failure also
        // removes the capability file and releases any workers already started.
        let server = Self {
            descriptor: descriptor.clone(),
            receiver,
            running: running.clone(),
            path,
            _lock: lock,
        };
        for index in 0..4 {
            let clients = clients.clone();
            let sender = sender.clone();
            let wake = wake.clone();
            let descriptor = descriptor.clone();
            let alive = running.clone();
            std::thread::Builder::new().name(format!("control-client-{index}")).spawn(move || {
                loop {
                    let next = clients.lock().expect("control connections lock").recv();
                    let Ok(mut stream) = next else { break; };
                    if !alive.load(Ordering::Relaxed) { break; }
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                    let Ok(bytes) = read_frame(&mut stream, MAX_FRAME, Duration::from_secs(1)) else { continue; };
                    let Ok(envelope) = serde_json::from_slice::<Envelope>(&bytes) else { continue; };
                    if envelope.token != descriptor.token || envelope.instance_id != descriptor.instance_id { continue; }
                    let id = envelope.request.request_id.clone();
                    let response = if envelope.request.schema_version != SCHEMA_VERSION {
                        Response::new(&id, &descriptor.instance_id, Err(ApiError::new("unsupported_version", "Unsupported request schema")))
                    } else {
                        let (reply, result) = mpsc::channel();
                        let pending = Pending { request: envelope.request, reply, expires: Instant::now() + Duration::from_secs(10) };
                        match sender.try_send(pending) {
                            Ok(()) => {
                                wake();
                                result.recv_timeout(Duration::from_secs(11)).unwrap_or_else(|_| Response::new(&id, &descriptor.instance_id, Err(ApiError::new("outcome_unknown", "Manager response deadline expired; query state before retrying"))))
                            }
                            Err(_) => Response::new(&id, &descriptor.instance_id, Err(ApiError::new("busy", "Management queue is full or stopping"))),
                        }
                    };
                    if let Ok(mut bytes) = serde_json::to_vec(&response) {
                        if bytes.len() > MAX_RESPONSE {
                            bytes = serde_json::to_vec(&Response::new(&id, &descriptor.instance_id, Err(ApiError::new("too_large", "Response exceeds 1 MiB; use paginated session list")))).unwrap();
                        }
                        bytes.push(b'\n');
                        let _ = stream.write_all(&bytes);
                    }
                }
            })?;
        }
        let alive = running.clone();
        std::thread::Builder::new()
            .name("control-listener".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if !alive.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        let _ = connections.try_send(stream);
                    }
                }
            })?;
        Ok(server)
    }
    /// The capability is inherited only by this manager's own session processes.
    pub fn environment(&self) -> [(String, String); 3] {
        [
            (
                "AI_TERMINAL_CONTROL_ENDPOINT".into(),
                self.descriptor.endpoint.clone(),
            ),
            (
                "AI_TERMINAL_CONTROL_TOKEN".into(),
                self.descriptor.token.clone(),
            ),
            (
                "AI_TERMINAL_CONTROL_INSTANCE".into(),
                self.descriptor.instance_id.clone(),
            ),
        ]
    }
}
impl Drop for ControlServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = TcpStream::connect(&self.descriptor.endpoint);
        // The exclusive lock prevents a new owner from replacing the descriptor yet.
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
fn protect(bytes: &[u8], encrypt: bool) -> std::io::Result<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    // No CRYPTPROTECT_LOCAL_MACHINE: another Windows user cannot decrypt it.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: Windows returned an owned cbData allocation, released after copying.
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(result)
}
#[cfg(not(windows))]
fn protect(_bytes: &[u8], _encrypt: bool) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Native management requires Windows DPAPI",
    ))
}
