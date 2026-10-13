// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Unofficial realme Buds Air7 control. No phone, cloud account, or audio driver required.
//! [`Client`] works with any timed, reliable byte stream implementing [`Transport`].
//! Protocol values come from realme Link captures; see `docs/protocol.md` for evidence.
pub mod advanced;
pub mod client;
pub mod codec;
pub mod commands;
pub mod model;
pub mod semantic_diagnostics;
pub mod semantics;
pub mod transport;
pub use client::Client;
pub use model::*;
pub use semantic_diagnostics::*;
pub use semantics::*;
pub use transport::Transport;

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
