// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Supported drivers. Adding a model is explicit; no shared wire protocol is assumed.
pub mod air7;
/// Stable identifiers for drivers implemented by this SDK, not a discovery result.
pub const DRIVER_IDS: &[&str] = &[air7::DRIVER_ID];
