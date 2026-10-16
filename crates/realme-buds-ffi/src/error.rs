// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use realme_buds::devices::air7;
use std::cell::RefCell;

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
