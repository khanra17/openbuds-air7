// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Air7 protocol and state.

pub const DRIVER_ID: &str = "realme.buds-air7";
pub const CONTROL_SERVICE: crate::transport::ServiceUuid = crate::transport::ServiceUuid([
    0x00, 0x00, 0x07, 0x9a, 0xd1, 0x02, 0x11, 0xe1, 0x9b, 0x23, 0x00, 0x02, 0x5b, 0x00, 0xa5, 0xa5,
]);
pub mod advanced;
pub mod client;
pub mod codec;
pub mod commands;
pub mod model;
pub mod semantic_diagnostics;
pub mod semantics;
pub use client::Client;
pub use model::*;
pub use semantic_diagnostics::*;
pub use semantics::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Bluetooth I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid protocol data: {0}")]
    Protocol(&'static str),
    #[error("earbuds rejected command 0x{command:04x}: status {status}")]
    Rejected { command: u16, status: u8 },
    #[error("command 0x{0:04x} is not advertised by these earbuds")]
    Unsupported(u16),
    #[error("request timed out; the setting may have changed; query it before retrying")]
    Timeout,
    #[error("operation cancelled after stop/cleanup")]
    Cancelled,
    #[error("invalid setting: {0}")]
    Invalid(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
