// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Transport-independent, explicitly model-scoped earbud SDK.
//! Start with [`devices::air7`]. There is no default device/protocol inference.
//! No GUI, native platform backend or foreign runtime is required by this crate.
pub mod devices;
pub mod transport;
pub use transport::Transport;
