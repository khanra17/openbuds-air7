// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Explicitly opt-in stream logging. Logs may contain device addresses/profile data.
use super::Transport;
use std::{
    io::{self, Read, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub struct Traced<T, W> {
    inner: T,
    log: W,
}
impl<T, W> Traced<T, W> {
    pub fn new(inner: T, log: W) -> Self {
        Self { inner, log }
    }
    pub fn into_inner(self) -> T {
        self.inner
    }
}
impl<T, W: Write> Traced<T, W> {
    fn record(&mut self, direction: &str, bytes: &[u8]) -> io::Result<()> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        write!(
            self.log,
            "{{\"time_ms\":{ms},\"direction\":\"{direction}\",\"hex\":\""
        )?;
        for b in bytes {
            write!(self.log, "{b:02x}")?;
        }
        writeln!(self.log, "\"}}")?;
        self.log.flush()
    }
}
impl<T: Read, W: Write> Read for Traced<T, W> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(b)?;
        self.record("rx", &b[..n])?;
        Ok(n)
    }
}
impl<T: Write, W: Write> Write for Traced<T, W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(b)?;
        self.record("tx", &b[..n])?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        self.log.flush()
    }
}
impl<T: Transport, W: Write + Send> Transport for Traced<T, W> {
    fn set_read_timeout(&mut self, t: Duration) -> io::Result<()> {
        self.inner.set_read_timeout(t)
    }
}
