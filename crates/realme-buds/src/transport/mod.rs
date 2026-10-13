// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use std::io::{self, Read, Write};
use std::time::Duration;
/// Stream reads MUST time out at the supplied duration for request deadlines to work.
pub trait Transport: Read + Write + Send {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()>;
}
pub mod trace;
#[cfg(windows)]
pub mod windows;
impl Transport for std::net::TcpStream {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        std::net::TcpStream::set_read_timeout(self, Some(timeout))
    }
}
