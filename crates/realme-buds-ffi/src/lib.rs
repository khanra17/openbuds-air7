// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Versioned C boundary. See include/openbuds.h for ownership/threading contracts.
#![deny(unsafe_op_in_unsafe_fn)]
mod buffer;
mod driver;
mod error;
mod session;
mod transport;
pub use buffer::BufferV1;
pub use error::ErrorInfo;
use error::{Failure, guard};
use realme_buds::devices::air7;
use std::{sync::atomic::Ordering, time::Duration};
pub use transport::TransportV1;
const MAX_INPUT: usize = 1024 * 1024;
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_id() -> Result<u64, Failure> {
    let counter = &NEXT_ID;
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or_else(|| Failure::new(error::INTERNAL, "handle space exhausted"))?;
        match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(current),
            Err(actual) => current = actual,
        }
    }
}

unsafe fn input<'a>(p: *const u8, n: usize) -> Result<&'a [u8], Failure> {
    if n > MAX_INPUT {
        return Err(Failure::new(
            error::INVALID_ARGUMENT,
            "input exceeds one MiB",
        ));
    }
    if n == 0 {
        return Ok(&[]);
    }
    if p.is_null() {
        return Err(Failure::new(error::INVALID_ARGUMENT, "null input"));
    }
    // SAFETY: callers promise a readable initialized n-byte region for this call only.
    Ok(unsafe { std::slice::from_raw_parts(p, n) })
}
unsafe fn text<'a>(p: *const u8, n: usize) -> Result<&'a str, Failure> {
    // SAFETY: same borrowed region contract as input.
    std::str::from_utf8(unsafe { input(p, n)? })
        .map_err(|_| Failure::new(error::INVALID_ARGUMENT, "expected UTF-8"))
}
unsafe fn output<T>(p: *mut T, value: T) -> Result<(), Failure> {
    if p.is_null() {
        return Err(Failure::new(error::INVALID_ARGUMENT, "null output"));
    }
    // SAFETY: caller supplies an aligned writable output for T, never aliased during the call.
    unsafe { p.write(value) };
    Ok(())
}

/// ABI major version. No runtime/device is initialized.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_abi_version() -> u32 {
    1
}

/// Supported explicit driver IDs and the SDK version; not a device discovery operation.
/// # Safety
/// `out` must point to writable BufferV1 storage with no outstanding owned buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_driver_catalog(out: *mut BufferV1) -> i32 {
    guard(|| {
        // SAFETY: checked output helper; this entrypoint's pointer contract applies.
        unsafe {
            output(out, BufferV1::default())?;
        }
        let data=serde_json::to_vec(&serde_json::json!({"abi_version":1,"sdk_version":env!("CARGO_PKG_VERSION"),"drivers":realme_buds::devices::DRIVER_IDS})).map_err(|e|Failure::new(error::INTERNAL,e.to_string()))?;
        unsafe {
            output(out, buffer::allocate(data)?)?;
        }
        Ok(())
    })
}

/// Create a selected model over foreign stream callbacks. No Bluetooth/stream I/O occurs here.
/// # Safety
/// Input/model bytes and the callback table must be valid for this call. The table must have
/// at least its size/version prefix; a valid full table transfers its callback context lease
/// after model/table validation, including release on subsequent construction failure.
/// Callback code/context must remain valid through successful close; callbacks cannot unwind.
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
            return Err(Failure::new(error::INVALID_ARGUMENT, "null callbacks"));
        }
        // SAFETY: prefix is required readable by the contract; do not read a short legacy struct.
        let prefix = unsafe { callbacks.cast::<[u32; 2]>().read() };
        if prefix[0] < std::mem::size_of::<TransportV1>() as u32 || prefix[1] != 1 {
            return Err(Failure::new(
                error::INVALID_ARGUMENT,
                "callback ABI size/version mismatch",
            ));
        }
        let callbacks = unsafe { callbacks.read() };
        callbacks.validate()?;
        let transport = Box::new(transport::ForeignTransport::new(callbacks));
        let id = session::register(driver::create(model, transport)?)?;
        unsafe {
            output(out, id)?;
        }
        Ok(())
    })
}

/// Open an explicitly selected model over native Windows RFCOMM. Unsupported elsewhere.
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
            // The model chooses the service; the Windows transport has no default device assumption.
            let stream = realme_buds_windows::RfcommStream::connect_service(
                address,
                driver::windows_service(model)?,
            )?;
            let id = session::register(driver::create(model, Box::new(stream))?)?;
            unsafe {
                output(out, id)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = address;
            Err(Failure::new(
                error::UNSUPPORTED,
                "native Windows transport is unavailable on this platform; supply callbacks",
            ))
        }
    })
}

/// Perform the model-specific handshake. Required before commands, not a model inference step.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_initialize(handle: u64) -> i32 {
    guard(|| session::with_driver(handle, |d, _| d.initialize()))
}
/// Set the I/O deadline; milliseconds must be nonzero. No command is sent.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_set_timeout(handle: u64, milliseconds: u32) -> i32 {
    guard(|| {
        session::with_driver(handle, |d, _| {
            d.set_timeout(Duration::from_millis(milliseconds.into()))
        })
    })
}
/// Execute one request in the selected driver's JSON schema. No generic wire codes are inferred.
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
        let data = session::with_driver(handle, |d, cancel| d.call(bytes, cancel))?;
        unsafe {
            output(out, buffer::allocate(data)?)?;
        }
        Ok(())
    })
}
/// Signal cancellation of an active cancellable sound helper. Never waits for the session lock.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_cancel(handle: u64) -> i32 {
    guard(|| {
        session::get(handle)?.cancel.store(true, Ordering::Relaxed);
        Ok(())
    })
}
/// Close after active calls have returned. BUSY leaves the handle valid; cancel/wait/retry close.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_close(handle: u64) -> i32 {
    guard(|| session::close(handle))
}

/// Release a buffer using the same DLL allocator; zeroed buffers are harmless to release.
/// # Safety
/// `out` is a valid writable BufferV1 from this library (or all zero). Do not use/copy data after release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_buffer_release(out: *mut BufferV1) -> i32 {
    guard(|| {
        if out.is_null() {
            return Err(Failure::new(error::INVALID_ARGUMENT, "null buffer"));
        }
        let b = unsafe { out.read() };
        buffer::release(b.id)?;
        unsafe {
            output(out, BufferV1::default())?;
        }
        Ok(())
    })
}
/// Copy the calling thread's last error. Reading it does not lose device status/command fields.
/// # Safety
/// Both outputs are writable, correctly aligned storage. `message` owns no prior allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_last_error(info: *mut ErrorInfo, message: *mut BufferV1) -> i32 {
    let last = error::last();
    guard(|| {
        unsafe {
            output(message, BufferV1::default())?;
            output(info, last.info)?;
            output(message, buffer::allocate(last.message.into_bytes())?)?;
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
/// Read the Air7 mode without JSON. Codes belong only to the explicitly selected Air7 driver.
/// # Safety
/// `out` points to writable Air7NoiseV1 storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ob_v1_air7_get_noise(handle: u64, out: *mut Air7NoiseV1) -> i32 {
    guard(|| {
        unsafe {
            output(out, Air7NoiseV1::default())?;
        }
        let n = session::with_driver(handle, |d, _| {
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
/// Set an explicitly named Air7 mode code; no other model is allowed to use this entrypoint.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_air7_set_noise(handle: u64, mode: u32) -> i32 {
    guard(|| {
        let mode = match mode {
            1 => air7::NoiseMode::Off,
            2 => air7::NoiseMode::Transparency,
            4 => air7::NoiseMode::Mild,
            8 => air7::NoiseMode::Maximum,
            16 => air7::NoiseMode::Moderate,
            32 => air7::NoiseMode::Adaptive,
            _ => {
                return Err(Failure::new(
                    error::INVALID_ARGUMENT,
                    "unknown Air7 noise mode",
                ));
            }
        };
        session::with_driver(handle, |d, _| {
            driver::initialized_air7(d)?.client.set_noise(mode)?;
            Ok(())
        })
    })
}
/// Set a named Air7 feature. Unknown experimental IDs use the explicitly confirmed JSON operation.
#[unsafe(no_mangle)]
pub extern "C" fn ob_v1_air7_set_feature(handle: u64, feature: u32, enabled: u32) -> i32 {
    guard(|| {
        if enabled > 1 {
            return Err(Failure::new(
                error::INVALID_ARGUMENT,
                "enabled must be zero or one",
            ));
        }
        let f = u8::try_from(feature)
            .ok()
            .and_then(air7::Feature::from_id)
            .ok_or_else(|| Failure::new(error::UNSUPPORTED, "unidentified Air7 feature"))?;
        session::with_driver(handle, |d, _| {
            driver::initialized_air7(d)?
                .client
                .set_feature(f, enabled != 0)?;
            Ok(())
        })
    })
}
