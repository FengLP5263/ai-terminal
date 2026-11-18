//! Prevent two GUI writers for the same metadata directory (new-version instances).
#[cfg(windows)]
pub struct ManagerLease(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl ManagerLease {
    pub fn acquire(directory: &std::path::Path) -> std::io::Result<Self> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError},
            System::Threading::CreateMutexW,
        };
        std::fs::create_dir_all(directory)?;
        let path = directory.canonicalize()?.to_string_lossy().to_lowercase();
        let hash = path.bytes().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        });
        let name: Vec<u16> = format!("Local\\AI-Terminal-Workspace-{hash:016x}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // Lifetime of the named object, not mutex ownership, provides the lease.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "An AI Terminal manager already owns this data directory. Use ai-terminal-cli or a different --data-dir.",
            ));
        }
        Ok(Self(handle))
    }
}
#[cfg(windows)]
impl Drop for ManagerLease {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(not(windows))]
pub struct ManagerLease;
#[cfg(not(windows))]
impl ManagerLease {
    pub fn acquire(_directory: &std::path::Path) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Native management requires Windows",
        ))
    }
}
