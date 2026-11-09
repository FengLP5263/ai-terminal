//! Cancellable Windows directory notifications; no periodic filesystem scans.
use super::service::Shared;
use std::{path::Path, sync::Arc};

#[cfg(windows)]
pub(super) struct Watcher {
    stop: windows_sys::Win32::Foundation::HANDLE,
    thread: Option<std::thread::JoinHandle<()>>,
    alive: Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(windows)]
// SAFETY: the stop event is owned here; only SetEvent and the joined worker use it.
unsafe impl Send for Watcher {}
#[cfg(windows)]
impl Watcher {
    pub fn is_alive(&self) -> bool {
        self.alive.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn start(path: &Path, metadata: bool, shared: Arc<Shared>) -> std::io::Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::*,
            Storage::FileSystem::*,
            System::{IO::*, Threading::*},
        };
        let name: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: nul-terminated path; all handles/buffers remain valid through cancellation.
        unsafe {
            let file = CreateFileW(
                name.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            );
            if file == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }
            let stop = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
            let event = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
            if stop.is_null() || event.is_null() {
                CloseHandle(file);
                if !stop.is_null() {
                    CloseHandle(stop);
                }
                if !event.is_null() {
                    CloseHandle(event);
                }
                return Err(std::io::Error::last_os_error());
            }
            let handles = (file as usize, stop as usize, event as usize);
            let (ready, receive_ready) = std::sync::mpsc::sync_channel(1);
            let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let running = alive.clone();
            let thread = std::thread::Builder::new()
                .name("git-directory-events".into())
                .spawn(move || {
                    let (file, stop, event) = (
                        handles.0 as HANDLE,
                        handles.1 as HANDLE,
                        handles.2 as HANDLE,
                    );
                    let mut buffer = vec![0u32; 16384];
                    let mut ready = Some(ready);
                    loop {
                        let mut overlapped: OVERLAPPED = std::mem::zeroed();
                        overlapped.hEvent = event;
                        ResetEvent(event);
                        if ReadDirectoryChangesW(
                            file,
                            buffer.as_mut_ptr().cast(),
                            65536,
                            1,
                            FILE_NOTIFY_CHANGE_FILE_NAME
                                | FILE_NOTIFY_CHANGE_DIR_NAME
                                | FILE_NOTIFY_CHANGE_LAST_WRITE
                                | FILE_NOTIFY_CHANGE_SIZE,
                            std::ptr::null_mut(),
                            &mut overlapped,
                            None,
                        ) == 0
                        {
                            if let Some(ready) = ready.take() {
                                let _ = ready.send(Err(std::io::Error::last_os_error()));
                            }
                            shared.changed(None);
                            break;
                        }
                        if let Some(ready) = ready.take() {
                            let _ = ready.send(Ok(()));
                        }
                        let wait = WaitForMultipleObjects(2, [stop, event].as_ptr(), 0, INFINITE);
                        if wait != WAIT_OBJECT_0 + 1 {
                            CancelIoEx(file, &overlapped);
                            let mut bytes = 0;
                            GetOverlappedResult(file, &overlapped, &mut bytes, 1);
                            break;
                        }
                        let mut bytes = 0;
                        if GetOverlappedResult(file, &overlapped, &mut bytes, 0) == 0 {
                            shared.changed(None);
                            break;
                        }
                        if bytes == 0 {
                            shared.changed(None);
                            continue;
                        }
                        if metadata {
                            shared.changed(None);
                            continue;
                        }
                        let data = std::slice::from_raw_parts(
                            buffer.as_ptr().cast::<u8>(),
                            bytes as usize,
                        );
                        let mut offset = 0;
                        while offset + 12 <= data.len() {
                            let next =
                                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
                                    as usize;
                            let len = u32::from_le_bytes(
                                data[offset + 8..offset + 12].try_into().unwrap(),
                            ) as usize;
                            if !len.is_multiple_of(2) || offset + 12 + len > data.len() {
                                shared.changed(None);
                                break;
                            }
                            let chars: Vec<u16> = data[offset + 12..offset + 12 + len]
                                .as_chunks::<2>()
                                .0
                                .iter()
                                .map(|s| u16::from_le_bytes([s[0], s[1]]))
                                .collect();
                            let name = String::from_utf16_lossy(&chars);
                            if !name
                                .split(['\\', '/'])
                                .next()
                                .is_some_and(|p| p.eq_ignore_ascii_case(".git"))
                            {
                                shared.changed(Some(std::path::PathBuf::from(name)));
                            }
                            if next == 0 {
                                break;
                            }
                            if next < 12 || offset + next > data.len() {
                                shared.changed(None);
                                break;
                            }
                            offset += next;
                        }
                    }
                    CloseHandle(file);
                    CloseHandle(event);
                    running.store(false, std::sync::atomic::Ordering::Release);
                });
            match thread {
                Ok(thread) => {
                    let watcher = Self {
                        stop,
                        thread: Some(thread),
                        alive,
                    };
                    match receive_ready.recv() {
                        Ok(Ok(())) => Ok(watcher),
                        Ok(Err(error)) => Err(error),
                        Err(_) => Err(std::io::Error::other("Directory monitor did not start")),
                    }
                }
                Err(error) => {
                    CloseHandle(file);
                    CloseHandle(event);
                    CloseHandle(stop);
                    Err(error)
                }
            }
        }
    }
}
#[cfg(windows)]
impl Drop for Watcher {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::SetEvent(self.stop);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.stop);
        }
    }
}
#[cfg(not(windows))]
pub(super) struct Watcher;
#[cfg(not(windows))]
impl Watcher {
    pub fn is_alive(&self) -> bool {
        false
    }
    pub fn start(_: &Path, _: bool, _: Arc<Shared>) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Windows filesystem events are unavailable",
        ))
    }
}
