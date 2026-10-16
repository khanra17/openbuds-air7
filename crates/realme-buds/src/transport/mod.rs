// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use std::io::{self, Read, Write};
use std::time::Duration;
/// Stream reads MUST time out at the supplied duration for request deadlines to work.
pub trait Transport: Read + Write + Send {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()>;
}
pub mod trace;
/// Standard 128-bit Bluetooth service UUID in network byte order. Not a device profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceUuid(pub [u8; 16]);
impl<T: Transport + ?Sized> Transport for Box<T> {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        (**self).set_read_timeout(timeout)
    }
}
impl Transport for std::net::TcpStream {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        std::net::TcpStream::set_read_timeout(self, Some(timeout))
    }
}
