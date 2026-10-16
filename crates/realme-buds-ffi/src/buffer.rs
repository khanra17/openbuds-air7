// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use crate::error::{self, Failure};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BufferV1 {
    pub id: u64,
    pub data: *const u8,
    pub len: usize,
}
impl Default for BufferV1 {
    fn default() -> Self {
        Self {
            id: 0,
            data: std::ptr::null(),
            len: 0,
        }
    }
}
static BUFFERS: LazyLock<Mutex<HashMap<u64, Box<[u8]>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn allocate(data: Vec<u8>) -> Result<BufferV1, Failure> {
    let id = crate::next_id()?;
    let data = data.into_boxed_slice();
    let result = BufferV1 {
        id,
        data: data.as_ptr(),
        len: data.len(),
    };
    BUFFERS
        .lock()
        .map_err(|_| Failure::new(error::INTERNAL, "buffer registry poisoned"))?
        .insert(id, data);
    Ok(result)
}
pub fn release(id: u64) -> Result<(), Failure> {
    if id == 0 {
        return Ok(());
    }
    BUFFERS
        .lock()
        .map_err(|_| Failure::new(error::INTERNAL, "buffer registry poisoned"))?
        .remove(&id)
        .ok_or_else(|| Failure::new(error::INVALID_HANDLE, "invalid or released buffer"))?;
    Ok(())
}
