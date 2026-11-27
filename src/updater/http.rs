//! Bounded HTTPS downloads using Windows' system proxy and certificate store.
use std::{
    ffi::c_void,
    io, ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::Networking::WinHttp::*;

struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(super) fn get(path: &str, limit: usize, mut report: impl FnMut(usize)) -> io::Result<Vec<u8>> {
    let began = Instant::now();
    let agent = wide(concat!("ai-terminal/", env!("CARGO_PKG_VERSION")));
    let host = wide("github.com");
    let mut status = 0u32;
    let mut status_size = std::mem::size_of::<u32>() as u32;
    let (request, _connection, _session) = unsafe {
        let session = Handle::new(WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        ))?;
        check(WinHttpSetTimeouts(session.0, 5_000, 5_000, 10_000, 15_000))?;
        let connection = Handle::new(WinHttpConnect(session.0, host.as_ptr(), 443, 0))?;
        let path = wide(path);
        let request = Handle::new(WinHttpOpenRequest(
            connection.0,
            ptr::null(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        let redirect = WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP;
        check(WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            (&redirect as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        ))?;
        check(WinHttpSendRequest(
            request.0,
            ptr::null(),
            0,
            ptr::null(),
            0,
            0,
            0,
        ))?;
        check(WinHttpReceiveResponse(request.0, ptr::null_mut()))?;
        check(WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut status_size,
            ptr::null_mut(),
        ))?;
        (request, connection, session)
    };
    if status != 200 {
        let kind = if status == 404 {
            io::ErrorKind::NotFound
        } else {
            io::ErrorKind::Other
        };
        return Err(io::Error::new(kind, format!("GitHub HTTP {status}")));
    }
    let mut bytes = Vec::with_capacity(limit.min(2 * 1024 * 1024));
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let mut read = 0u32;
        check(unsafe {
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            )
        })?;
        if read == 0 {
            return Ok(bytes);
        }
        let read = read as usize;
        if bytes.len() + read > limit || began.elapsed() >= Duration::from_secs(180) {
            return Err(io::Error::other(
                "Update response exceeded its size or time limit",
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
        report(bytes.len());
    }
}
