// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use crate::{
    driver::Driver,
    error::{self, Failure},
};
use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Session {
    pub driver: Mutex<Option<Box<dyn Driver>>>,
    pub cancel: Arc<AtomicBool>,
}
static SESSIONS: LazyLock<Mutex<HashMap<u64, Arc<Session>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn register(driver: Box<dyn Driver>) -> Result<u64, Failure> {
    let id = crate::next_id()?;
    let session = Arc::new(Session {
        driver: Mutex::new(Some(driver)),
        cancel: Arc::new(AtomicBool::new(false)),
    });
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(error::INTERNAL, "session registry poisoned"))?
        .insert(id, session);
    Ok(id)
}
pub fn get(id: u64) -> Result<Arc<Session>, Failure> {
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(error::INTERNAL, "session registry poisoned"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| Failure::new(error::INVALID_HANDLE, "invalid or closed session"))
}
pub fn with_driver<T>(
    id: u64,
    f: impl FnOnce(&mut dyn Driver, &AtomicBool) -> Result<T, Failure>,
) -> Result<T, Failure> {
    let session = get(id)?;
    let mut guard = session.driver.try_lock().map_err(|e| match e {
        std::sync::TryLockError::WouldBlock => Failure::new(
            error::BUSY,
            "session already in use; callbacks may not reenter it",
        ),
        std::sync::TryLockError::Poisoned(_) => Failure::new(
            error::INTERNAL,
            "session panicked; close it before reconnecting",
        ),
    })?;
    let driver = guard
        .as_mut()
        .ok_or_else(|| Failure::new(error::INVALID_HANDLE, "closed session"))?;
    session.cancel.store(false, Ordering::Relaxed);
    f(driver.as_mut(), &session.cancel)
}
pub fn close(id: u64) -> Result<(), Failure> {
    let session = get(id)?;
    let mut guard = match session.driver.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => {
            return Err(Failure::new(
                error::BUSY,
                "cancel/wait for the active call before closing",
            ));
        }
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
    };
    let driver = guard
        .take()
        .ok_or_else(|| Failure::new(error::INVALID_HANDLE, "closed session"))?;
    SESSIONS
        .lock()
        .map_err(|_| Failure::new(error::INTERNAL, "session registry poisoned"))?
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
                return Err(Failure::new(error::CANCELLED, "cancelled"));
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
        assert_eq!(crate::ob_v1_close(h), error::BUSY);
        assert_eq!(crate::ob_v1_cancel(h), 0);
        assert_eq!(worker.join().unwrap(), error::CANCELLED);
        let mut out = crate::Air7NoiseV1::default();
        assert_eq!(
            unsafe { crate::ob_v1_air7_get_noise(h, &mut out) },
            error::UNKNOWN_DRIVER
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
            error::PANIC
        );
        assert_eq!(out.id, 0);
        assert_eq!(
            unsafe { crate::ob_v1_call(h, b"{}".as_ptr(), 2, &mut out) },
            error::INTERNAL
        );
        assert_eq!(crate::ob_v1_close(h), 0);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }
}
