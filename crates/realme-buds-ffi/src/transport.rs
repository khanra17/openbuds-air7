// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use crate::error::{self, Failure};
use realme_buds::Transport;
use std::{
    ffi::c_void,
    io::{self, Read, Write},
    time::Duration,
};

pub type ReadFn = unsafe extern "C" fn(*mut c_void, *mut u8, usize, u32, *mut usize) -> i32;
pub type WriteFn = unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut usize) -> i32;
pub type FlushFn = unsafe extern "C" fn(*mut c_void) -> i32;
pub type ReleaseFn = unsafe extern "C" fn(*mut c_void);
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TransportV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub context: *mut c_void,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
    pub flush: Option<FlushFn>,
    pub release: Option<ReleaseFn>,
}
impl TransportV1 {
    pub fn validate(&self) -> Result<(), Failure> {
        if self.abi_version != 1
            || self.struct_size < std::mem::size_of::<Self>() as u32
            || self.read.is_none()
            || self.write.is_none()
        {
            return Err(Failure::new(
                error::INVALID_ARGUMENT,
                "transport v1 size/version/read/write invalid",
            ));
        }
        Ok(())
    }
}
pub struct ForeignTransport {
    callbacks: TransportV1,
    timeout: u32,
}
impl ForeignTransport {
    pub fn new(callbacks: TransportV1) -> Self {
        Self {
            callbacks,
            timeout: 12000,
        }
    }
}
// SAFETY: the C contract requires context/callbacks to be usable from the invoking thread.
// Calls for a session are serialized; no two transport callbacks run concurrently.
unsafe impl Send for ForeignTransport {}
fn status(code: i32) -> io::Result<()> {
    match code {
        error::OK => Ok(()),
        error::TIMEOUT => Err(io::ErrorKind::TimedOut.into()),
        _ => Err(io::Error::other(format!(
            "foreign transport failed with status {code}"
        ))),
    }
}
impl Read for ForeignTransport {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;
        // SAFETY: validated callback, live context and temporary writable slice valid for this call.
        status(unsafe {
            self.callbacks.read.expect("validated read")(
                self.callbacks.context,
                b.as_mut_ptr(),
                b.len(),
                self.timeout,
                &mut n,
            )
        })?;
        if n > b.len() {
            return Err(io::Error::other("foreign read count exceeds capacity"));
        }
        Ok(n)
    }
}
impl Write for ForeignTransport {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut n = 0;
        // SAFETY: validated callback receives a borrowed readable slice, never Rust ownership.
        status(unsafe {
            self.callbacks.write.expect("validated write")(
                self.callbacks.context,
                b.as_ptr(),
                b.len(),
                &mut n,
            )
        })?;
        if n > b.len() {
            return Err(io::Error::other("foreign write count exceeds input"));
        }
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.callbacks.flush {
            // SAFETY: caller guarantees the callback/context remain live until successful close.
            status(unsafe { f(self.callbacks.context) })?;
        }
        Ok(())
    }
}
impl Transport for ForeignTransport {
    fn set_read_timeout(&mut self, d: Duration) -> io::Result<()> {
        self.timeout = d.as_millis().clamp(1, u32::MAX as u128) as u32;
        Ok(())
    }
}
impl Drop for ForeignTransport {
    fn drop(&mut self) {
        if let Some(f) = self.callbacks.release {
            // SAFETY: ownership is released exactly once; caller's callback must not unwind.
            unsafe { f(self.callbacks.context) };
        }
    }
}
