// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Native WinSock Bluetooth Classic RFCOMM. Pair through Windows Settings first.
#![cfg(windows)]
use realme_buds::{Transport, transport::ServiceUuid};
use std::{
    io::{self, Read, Write},
    mem::size_of,
    time::Duration,
};
use windows_sys::{
    Win32::{Devices::Bluetooth::*, Networking::WinSock::*},
    core::GUID,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_uuid_uses_network_order_fields_without_a_model_default() {
        let value = guid(ServiceUuid([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]));
        assert_eq!(value.data1, 0x01020304);
        assert_eq!(value.data2, 0x0506);
        assert_eq!(value.data3, 0x0708);
        assert_eq!(value.data4, [9, 10, 11, 12, 13, 14, 15, 16]);
    }
    #[test]
    fn address_roundtrip_is_separate_from_service_selection() {
        let synthetic = (1..=6)
            .map(|n| format!("{n:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        assert_eq!(
            format_address(parse_address(&synthetic).unwrap()),
            synthetic
        );
        assert!(parse_address("not-an-address").is_err());
    }
}

fn guid(uuid: ServiceUuid) -> GUID {
    GUID {
        data1: u32::from_be_bytes(uuid.0[0..4].try_into().expect("four bytes")),
        data2: u16::from_be_bytes([uuid.0[4], uuid.0[5]]),
        data3: u16::from_be_bytes([uuid.0[6], uuid.0[7]]),
        data4: uuid.0[8..16].try_into().expect("eight bytes"),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PairedDevice {
    pub name: String,
    pub address: String,
    pub connected: bool,
}
/// Enumerate remembered/authenticated devices, without an active discovery inquiry.
pub fn paired_devices() -> io::Result<Vec<PairedDevice>> {
    let params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
        fReturnAuthenticated: 1,
        fReturnRemembered: 1,
        fReturnConnected: 1,
        ..Default::default()
    };
    let mut info = BLUETOOTH_DEVICE_INFO {
        dwSize: size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    // SAFETY: sized, initialized Win32 structures remain alive throughout synchronous calls.
    unsafe {
        let h = BluetoothFindFirstDevice(&params, &mut info);
        if h.is_null() {
            let e = io::Error::last_os_error();
            return if e.raw_os_error() == Some(259) {
                Ok(Vec::new())
            } else {
                Err(e)
            };
        }
        let mut result = Vec::new();
        loop {
            let n = info
                .szName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szName.len());
            let a = info.Address.Anonymous.ullLong;
            result.push(PairedDevice {
                name: String::from_utf16_lossy(&info.szName[..n]),
                address: format_address(a),
                connected: info.fConnected != 0,
            });
            if BluetoothFindNextDevice(h, &mut info) == 0 {
                break;
            }
        }
        let error = io::Error::last_os_error();
        BluetoothFindDeviceClose(h);
        if error.raw_os_error() != Some(259) {
            return Err(error);
        }
        Ok(result)
    }
}
pub fn parse_address(address: &str) -> io::Result<u64> {
    let p: Vec<_> = address.split(':').collect();
    if p.len() != 6 || p.iter().any(|s| s.len() != 2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected XX:XX:XX:XX:XX:XX",
        ));
    }
    let mut a = 0;
    for s in p {
        a = (a << 8)
            | u8::from_str_radix(s, 16).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid Bluetooth address")
            })? as u64;
    }
    Ok(a)
}
fn format_address(a: u64) -> String {
    (0..6)
        .rev()
        .map(|i| format!("{:02X}", (a >> (i * 8)) & 255))
        .collect::<Vec<_>>()
        .join(":")
}
fn socket_error() -> io::Error {
    // SAFETY: WSAGetLastError has no pointer arguments; capture it immediately.
    let code = unsafe { WSAGetLastError() };
    if code == WSAETIMEDOUT {
        io::Error::new(io::ErrorKind::TimedOut, "Bluetooth timeout")
    } else {
        io::Error::from_raw_os_error(code)
    }
}
struct Winsock;
impl Winsock {
    fn start() -> io::Result<Self> {
        let mut data = WSADATA::default();
        // SAFETY: data points to a valid WSADATA; this guard balances WSAStartup.
        let rc = unsafe { WSAStartup(0x0202, &mut data) };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        Ok(Self)
    }
}
impl Drop for Winsock {
    fn drop(&mut self) {
        unsafe {
            WSACleanup();
        }
    }
}
pub struct RfcommStream {
    socket: SOCKET,
    _winsock: Winsock,
}
impl RfcommStream {
    /// The caller/driver supplies a service; this platform adapter selects no model.
    pub fn connect_service(address: &str, service: ServiceUuid) -> io::Result<Self> {
        Self::open(address, guid(service), 0)
    }
    /// Diagnostic fallback if Windows SDP does not resolve the UUID. Do not guess a channel.
    pub fn connect_channel(address: &str, channel: u32) -> io::Result<Self> {
        if !(1..=30).contains(&channel) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RFCOMM channel must be 1..30",
            ));
        }
        Self::open(address, GUID::default(), channel)
    }
    fn open(address: &str, uuid: GUID, port: u32) -> io::Result<Self> {
        let a = parse_address(address)?;
        let winsock = Winsock::start()?;
        // SAFETY: socket parameters specify the native Windows RFCOMM provider.
        let s = unsafe { socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM as i32) };
        if s == INVALID_SOCKET {
            return Err(socket_error());
        }
        let mut stream = Self {
            socket: s,
            _winsock: winsock,
        };
        let addr = SOCKADDR_BTH {
            addressFamily: AF_BTH,
            btAddr: a,
            serviceClassId: uuid,
            port,
        };
        // SAFETY: SOCKADDR_BTH is the Windows packed ABI; size is supplied explicitly.
        if unsafe {
            connect(
                s,
                (&addr as *const SOCKADDR_BTH).cast(),
                size_of::<SOCKADDR_BTH>() as i32,
            )
        } == SOCKET_ERROR
        {
            return Err(socket_error());
        }
        stream.set_read_timeout(Duration::from_secs(12))?;
        stream.timeout_option(SO_SNDTIMEO, Duration::from_secs(12))?;
        Ok(stream)
    }
    fn timeout_option(&self, option: i32, timeout: Duration) -> io::Result<()> {
        let ms = timeout.as_millis().clamp(1, u32::MAX as u128) as u32;
        // SAFETY: WinSock timeout value is a DWORD; pointer and byte size are valid.
        if unsafe {
            setsockopt(
                self.socket,
                SOL_SOCKET,
                option,
                (&ms as *const u32).cast(),
                4,
            )
        } == SOCKET_ERROR
        {
            Err(socket_error())
        } else {
            Ok(())
        }
    }
}
impl Read for RfcommStream {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if b.is_empty() {
            return Ok(0);
        }
        // SAFETY: writable buffer lives for this blocking call; length is bounded to i32.
        let n = unsafe {
            recv(
                self.socket,
                b.as_mut_ptr(),
                b.len().min(i32::MAX as usize) as i32,
                0,
            )
        };
        if n == SOCKET_ERROR {
            Err(socket_error())
        } else {
            Ok(n as usize)
        }
    }
}
impl Write for RfcommStream {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.is_empty() {
            return Ok(0);
        }
        // SAFETY: readable buffer lives for this blocking call.
        let n = unsafe {
            send(
                self.socket,
                b.as_ptr(),
                b.len().min(i32::MAX as usize) as i32,
                0,
            )
        };
        if n == SOCKET_ERROR {
            Err(socket_error())
        } else {
            Ok(n as usize)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Transport for RfcommStream {
    fn set_read_timeout(&mut self, t: Duration) -> io::Result<()> {
        self.timeout_option(SO_RCVTIMEO, t)
    }
}
impl Drop for RfcommStream {
    fn drop(&mut self) {
        // SAFETY: this object owns this socket exclusively.
        unsafe {
            closesocket(self.socket);
        }
    }
}
