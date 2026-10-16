// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
#![deny(unsafe_op_in_unsafe_fn)]
mod air7;
mod driver;
mod runtime;
mod transport;
use realme_buds::devices::air7 as model;
pub use runtime::{BufferV1, ErrorInfo};
use runtime::{Failure, guard};
use std::{sync::atomic::Ordering, time::Duration};
pub use transport::TransportV1;
const MAX_INPUT: usize = 1024 * 1024;

unsafe fn input<'a>(p: *const u8, n: usize) -> Result<&'a [u8], Failure> {
    if n > MAX_INPUT {
        return Err(Failure::new(
            runtime::INVALID_ARGUMENT,
            "input exceeds one MiB",
        ));
    }
    if n == 0 {
        return Ok(&[]);
    }
    if p.is_null() {
        return Err(Failure::new(runtime::INVALID_ARGUMENT, "null input"));
    }
    // SAFETY: callers promise a readable initialized n-byte region for this call only.
    Ok(unsafe { std::slice::from_raw_parts(p, n) })
}
unsafe fn text<'a>(p: *const u8, n: usize) -> Result<&'a str, Failure> {
    // SAFETY: same borrowed region contract as input.
    std::str::from_utf8(unsafe { input(p, n)? })
        .map_err(|_| Failure::new(runtime::INVALID_ARGUMENT, "expected UTF-8"))
}
unsafe fn output<T>(p: *mut T, value: T) -> Result<(), Failure> {
    if p.is_null() {
        return Err(Failure::new(runtime::INVALID_ARGUMENT, "null output"));
    }
    // SAFETY: caller supplies an aligned writable output for T, never aliased during the call.
    unsafe { p.write(value) };
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_abi_version() -> u32 {
    1
}

/// # Safety
/// `out` must point to writable BufferV1 storage with no outstanding owned buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_driver_catalog(out: *mut BufferV1) -> i32 {
    guard(|| {
        unsafe {
            output(out, BufferV1::default())?;
        }
        let catalog = serde_json::json!({
            "abi_version": 1,
            "sdk_version": env!("CARGO_PKG_VERSION"),
            "drivers": realme_buds::devices::DRIVER_IDS,
            "connection_metadata": {
                (model::DRIVER_ID): {
                    "kind": "rfcomm",
                    "service_uuid_be": model::CONTROL_SERVICE.0
                }
            }
        });
        let data = serde_json::to_vec(&catalog)
            .map_err(|e| Failure::new(runtime::INTERNAL, e.to_string()))?;
        unsafe {
            output(out, runtime::allocate_buffer(data)?)?;
        }
        Ok(())
    })
}

/// # Safety
/// `out` is writable BufferV1 storage with no outstanding allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_windows_paired_devices(out: *mut BufferV1) -> i32 {
    guard(|| {
        unsafe {
            output(out, BufferV1::default())?;
        }
        #[cfg(windows)]
        {
            let devices = realme_buds_windows::paired_devices()?;
            let data = serde_json::to_vec(&serde_json::json!({"abi_version":1,"devices":devices}))
                .map_err(|e| Failure::new(runtime::INTERNAL, e.to_string()))?;
            unsafe {
                output(out, runtime::allocate_buffer(data)?)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(Failure::new(
                runtime::UNSUPPORTED,
                "native Windows enumeration is unavailable on this platform",
            ))
        }
    })
}

/// # Safety
/// Inputs and the table's size/version prefix must be readable; `out` must be writable.
/// Accepted callbacks/context must remain live until close and must not unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_open(
    model: *const u8,
    model_len: usize,
    callbacks: *const TransportV1,
    out: *mut u64,
) -> i32 {
    guard(|| {
        unsafe {
            output(out, 0)?;
        }
        let model = unsafe { text(model, model_len)? };
        driver::validate_model(model)?;
        if callbacks.is_null() {
            return Err(Failure::new(runtime::INVALID_ARGUMENT, "null callbacks"));
        }
        // SAFETY: prefix is required readable by the contract; do not read a short legacy struct.
        let prefix = unsafe { callbacks.cast::<[u32; 2]>().read() };
        if prefix[0] < std::mem::size_of::<TransportV1>() as u32 || prefix[1] != 1 {
            return Err(Failure::new(
                runtime::INVALID_ARGUMENT,
                "callback ABI size/version mismatch",
            ));
        }
        let callbacks = unsafe { callbacks.read() };
        callbacks.validate()?;
        let transport = Box::new(transport::ForeignTransport::new(callbacks));
        let id = runtime::register(driver::create(model, transport)?)?;
        unsafe {
            output(out, id)?;
        }
        Ok(())
    })
}

/// # Safety
/// Both input slices must be valid UTF-8 and `out` writable for one handle. No strings are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_open_windows(
    model: *const u8,
    model_len: usize,
    address: *const u8,
    address_len: usize,
    out: *mut u64,
) -> i32 {
    guard(|| {
        unsafe {
            output(out, 0)?;
        }
        let model = unsafe { text(model, model_len)? };
        driver::validate_model(model)?;
        let address = unsafe { text(address, address_len)? };
        #[cfg(windows)]
        {
            let stream = realme_buds_windows::RfcommStream::connect_service(
                address,
                driver::windows_service(model)?,
            )?;
            let id = runtime::register(driver::create(model, Box::new(stream))?)?;
            unsafe {
                output(out, id)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = address;
            Err(Failure::new(
                runtime::UNSUPPORTED,
                "native Windows transport is unavailable on this platform; supply callbacks",
            ))
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_initialize(handle: u64) -> i32 {
    guard(|| runtime::with_driver(handle, |d, _| d.initialize()))
}
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_set_timeout(handle: u64, milliseconds: u32) -> i32 {
    guard(|| {
        runtime::with_driver(handle, |d, _| {
            d.set_timeout(Duration::from_millis(milliseconds.into()))
        })
    })
}
/// # Safety
/// Request is a valid readable byte slice; `out` is writable and holds no owned allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_call(
    handle: u64,
    request: *const u8,
    len: usize,
    out: *mut BufferV1,
) -> i32 {
    guard(|| {
        unsafe {
            output(out, BufferV1::default())?;
        }
        let bytes = unsafe { input(request, len)? };
        let data = runtime::with_driver(handle, |d, cancel| d.call(bytes, cancel))?;
        unsafe {
            output(out, runtime::allocate_buffer(data)?)?;
        }
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_cancel(handle: u64) -> i32 {
    guard(|| {
        runtime::get(handle)?.cancel.store(true, Ordering::Relaxed);
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_close(handle: u64) -> i32 {
    guard(|| runtime::close(handle))
}

/// # Safety
/// `out` is a valid writable BufferV1 from this library (or all zero). Do not use/copy data after release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_buffer_release(out: *mut BufferV1) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(Failure::new(runtime::INVALID_ARGUMENT, "null buffer"));
        }
        let b = unsafe { out.read() };
        runtime::release_buffer(b.id)?;
        unsafe {
            output(out, BufferV1::default())?;
        }
        Ok(())
    })
}
/// # Safety
/// Both outputs are writable, correctly aligned storage. `message` owns no prior allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_last_error(info: *mut ErrorInfo, message: *mut BufferV1) -> i32 {
    let last = runtime::last();
    guard(|| {
        unsafe {
            output(message, BufferV1::default())?;
            output(info, last.info)?;
            output(
                message,
                runtime::allocate_buffer(last.message.into_bytes())?,
            )?;
        }
        Ok(())
    })
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Air7NoiseV1 {
    pub action: u8,
    pub kind: u8,
    pub value: u8,
    pub reserved: u8,
}
/// # Safety
/// `out` points to writable Air7NoiseV1 storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_air7_get_noise(handle: u64, out: *mut Air7NoiseV1) -> i32 {
    guard(|| {
        unsafe {
            output(out, Air7NoiseV1::default())?;
        }
        let n = runtime::with_driver(handle, |d, _| {
            Ok(driver::initialized_air7(d)?.client.noise()?)
        })?;
        unsafe {
            output(
                out,
                Air7NoiseV1 {
                    action: n.action,
                    kind: n.kind,
                    value: n.value,
                    reserved: 0,
                },
            )?;
        }
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_air7_set_noise(handle: u64, mode: u32) -> i32 {
    guard(|| {
        let mode = match mode {
            1 => model::NoiseMode::Off,
            2 => model::NoiseMode::Transparency,
            4 => model::NoiseMode::Mild,
            8 => model::NoiseMode::Maximum,
            16 => model::NoiseMode::Moderate,
            32 => model::NoiseMode::Adaptive,
            _ => {
                return Err(Failure::new(
                    runtime::INVALID_ARGUMENT,
                    "unknown Air7 noise mode",
                ));
            }
        };
        runtime::with_driver(handle, |d, _| {
            driver::initialized_air7(d)?.client.set_noise(mode)?;
            Ok(())
        })
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_air7_set_feature(handle: u64, feature: u32, enabled: u32) -> i32 {
    guard(|| {
        if enabled > 1 {
            return Err(Failure::new(
                runtime::INVALID_ARGUMENT,
                "enabled must be zero or one",
            ));
        }
        let f = u8::try_from(feature)
            .ok()
            .and_then(model::Feature::from_id)
            .ok_or_else(|| Failure::new(runtime::UNSUPPORTED, "unidentified Air7 feature"))?;
        runtime::with_driver(handle, |d, _| {
            driver::initialized_air7(d)?
                .client
                .set_feature(f, enabled != 0)?;
            Ok(())
        })
    })
}
