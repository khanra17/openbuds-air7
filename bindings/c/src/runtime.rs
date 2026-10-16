// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use crate::driver::Driver;
use realme_buds::devices::air7;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn next_id() -> Result<u64, Failure> {
    let counter = &NEXT_ID;
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or_else(|| Failure::new(INTERNAL, "handle space exhausted"))?;
        match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(current),
            Err(actual) => current = actual,
        }
    }
}

pub const OK: i32 = 0;
pub const INVALID_ARGUMENT: i32 = 1;
pub const UNKNOWN_DRIVER: i32 = 2;
pub const INVALID_HANDLE: i32 = 3;
pub const TIMEOUT: i32 = 4;
pub const IO: i32 = 5;
pub const REJECTED: i32 = 6;
pub const UNSUPPORTED: i32 = 7;
pub const BUSY: i32 = 8;
pub const CANCELLED: i32 = 9;
pub const NOT_INITIALIZED: i32 = 10;
pub const PROTOCOL: i32 = 11;
pub const INTERNAL: i32 = 254;
pub const PANIC: i32 = 255;
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ErrorInfo {
    pub status: i32,
    pub command: u32,
    pub device_status: u32,
    pub has_device_status: u32,
}
#[derive(Debug, Clone, Default)]
pub struct Failure {
    pub info: ErrorInfo,
    pub message: String,
}
impl Failure {
    pub fn new(status: i32, message: impl Into<String>) -> Self {
        Self {
            info: ErrorInfo {
                status,
                ..Default::default()
            },
            message: message.into(),
        }
    }
}
impl From<air7::Error> for Failure {
    fn from(e: air7::Error) -> Self {
        let status = match &e {
            air7::Error::Io(_) => IO,
            air7::Error::Protocol(_) => PROTOCOL,
            air7::Error::Rejected { .. } => REJECTED,
            air7::Error::Unsupported(_) => UNSUPPORTED,
            air7::Error::Timeout => TIMEOUT,
            air7::Error::Cancelled => CANCELLED,
            air7::Error::Invalid(_) => INVALID_ARGUMENT,
        };
        let mut f = Self::new(status, e.to_string());
        match e {
            air7::Error::Rejected { command, status } => {
                f.info.command = command.into();
                f.info.device_status = status.into();
                f.info.has_device_status = 1;
            }
            air7::Error::Unsupported(c) => f.info.command = c.into(),
            _ => {}
        }
        f
    }
}
impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        Self::new(IO, e.to_string())
    }
}
thread_local! {static LAST:RefCell<Failure>=RefCell::new(Failure::default());}
pub fn last() -> Failure {
    LAST.with(|v| v.borrow().clone())
}
pub fn guard(f: impl FnOnce() -> Result<(), Failure>) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(())) => {
            LAST.with(|v| *v.borrow_mut() = Failure::default());
            OK
        }
        Ok(Err(e)) => {
            let status = e.info.status;
            LAST.with(|v| *v.borrow_mut() = e);
            status
        }
        Err(_) => {
            LAST.with(|v| {
                *v.borrow_mut() = Failure::new(PANIC, "Rust panic contained at the ABI boundary")
            });
            PANIC
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BufferV1 {
    pub id: u64,
    pub data: *const u8,
    pub len: usize,
}
impl Default for BufferV1 {
    fn default() -> Self {
        Self {
            id: 0,
            data: std::ptr::null(),
            len: 0,
        }
    }
}
static BUFFERS: LazyLock<Mutex<HashMap<u64, Box<[u8]>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn allocate_buffer(data: Vec<u8>) -> Result<BufferV1, Failure> {
    let id = next_id()?;
    let data = data.into_boxed_slice();
    let result = BufferV1 {
        id,
        data: data.as_ptr(),
        len: data.len(),
    };
    BUFFERS
        .lock()
        .map_err(|_| Failure::new(INTERNAL, "buffer registry poisoned"))?
        .insert(id, data);
    Ok(result)
}
pub fn release_buffer(id: u64) -> Result<(), Failure> {
    if id == 0 {
        return Ok(());
    }
    BUFFERS
        .lock()
        .map_err(|_| Failure::new(INTERNAL, "buffer registry poisoned"))?
        .remove(&id)
        .ok_or_else(|| Failure::new(INVALID_HANDLE, "invalid or released buffer"))?;
    Ok(())
}

pub struct Session {
    pub driver: Mutex<Option<Box<dyn Driver>>>,
    pub cancel: Arc<AtomicBool>,
}
static SESSIONS: LazyLock<Mutex<HashMap<u64, Arc<Session>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn register(driver: Box<dyn Driver>) -> Result<u64, Failure> {
    let id = next_id()?;
    let session = Arc::new(Session {
        driver: Mutex::new(Some(driver)),
        cancel: Arc::new(AtomicBool::new(false)),
    });
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(INTERNAL, "session registry poisoned"))?
        .insert(id, session);
    Ok(id)
}
pub fn get(id: u64) -> Result<Arc<Session>, Failure> {
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(INTERNAL, "session registry poisoned"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| Failure::new(INVALID_HANDLE, "invalid or closed session"))
}
pub fn with_driver<T>(
    id: u64,
    f: impl FnOnce(&mut dyn Driver, &AtomicBool) -> Result<T, Failure>,
) -> Result<T, Failure> {
    let session = get(id)?;
    let mut guard = session.driver.try_lock().map_err(|e| match e {
        std::sync::TryLockError::WouldBlock => {
            Failure::new(BUSY, "session already in use; callbacks may not reenter it")
        }
        std::sync::TryLockError::Poisoned(_) => {
            Failure::new(INTERNAL, "session panicked; close it before reconnecting")
        }
    })?;
    let driver = guard
        .as_mut()
        .ok_or_else(|| Failure::new(INVALID_HANDLE, "closed session"))?;
    session.cancel.store(false, Ordering::Relaxed);
    f(driver.as_mut(), &session.cancel)
}
pub fn close(id: u64) -> Result<(), Failure> {
    let session = get(id)?;
    let mut guard = match session.driver.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => {
            return Err(Failure::new(
                BUSY,
                "cancel/wait for the active call before closing",
            ));
        }
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
    };
    let driver = guard
        .take()
        .ok_or_else(|| Failure::new(INVALID_HANDLE, "closed session"))?;
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(INTERNAL, "session registry poisoned"))?
        .remove(&id);
    drop(guard);
    // Release callbacks run outside both locks and finish before close returns.
    drop(driver);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        any::Any,
        sync::{Barrier, atomic::AtomicUsize},
        time::{Duration, Instant},
    };
    struct OtherDriver {
        entered: Option<Arc<Barrier>>,
        panic: bool,
        dropped: Arc<AtomicUsize>,
    }
    impl Driver for OtherDriver {
        fn initialize(&mut self) -> Result<(), Failure> {
            Ok(())
        }
        fn set_timeout(&mut self, _: Duration) -> Result<(), Failure> {
            Ok(())
        }
        fn call(&mut self, _: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>, Failure> {
            assert!(!self.panic, "injected driver panic");
            if let Some(b) = &self.entered {
                b.wait();
                let deadline = Instant::now() + Duration::from_secs(3);
                while !cancel.load(Ordering::Relaxed) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(1));
                }
                return Err(Failure::new(CANCELLED, "cancelled"));
            }
            Ok(b"{}".to_vec())
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl Drop for OtherDriver {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[test]
    fn busy_cancel_close_and_model_isolation_are_independent_of_air7() {
        let barrier = Arc::new(Barrier::new(2));
        let dropped = Arc::new(AtomicUsize::new(0));
        let h = register(Box::new(OtherDriver {
            entered: Some(barrier.clone()),
            panic: false,
            dropped: dropped.clone(),
        }))
        .unwrap();
        let worker = std::thread::spawn(move || {
            let mut out = crate::BufferV1::default();
            // SAFETY: valid borrowed request/output. Driver deliberately blocks until cancel.
            unsafe { crate::ob_v1_call(h, b"{}".as_ptr(), 2, &mut out) }
        });
        barrier.wait();
        assert_eq!(crate::ob_v1_close(h), BUSY);
        assert_eq!(crate::ob_v1_cancel(h), 0);
        assert_eq!(worker.join().unwrap(), CANCELLED);
        let mut out = crate::Air7NoiseV1::default();
        assert_eq!(
            unsafe { crate::ob_v1_air7_get_noise(h, &mut out) },
            UNKNOWN_DRIVER
        );
        assert_eq!(crate::ob_v1_close(h), 0);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn panic_is_contained_and_poisoned_session_still_closes() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let h = register(Box::new(OtherDriver {
            entered: None,
            panic: true,
            dropped: dropped.clone(),
        }))
        .unwrap();
        let mut out = crate::BufferV1::default();
        assert_eq!(
            unsafe { crate::ob_v1_call(h, b"{}".as_ptr(), 2, &mut out) },
            PANIC
        );
        assert_eq!(out.id, 0);
        assert_eq!(
            unsafe { crate::ob_v1_call(h, b"{}".as_ptr(), 2, &mut out) },
            INTERNAL
        );
        assert_eq!(crate::ob_v1_close(h), 0);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }
}
