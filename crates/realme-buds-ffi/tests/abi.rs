// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Offline ABI contract tests. All transport bytes are synthetic.
use openbuds::*;
use realme_buds::devices::air7::codec::{Decoder, Packet, encode};
use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
struct Context {
    decoder: Decoder,
    input: VecDeque<u8>,
    released: Arc<AtomicUsize>,
    bad_count: bool,
}
unsafe extern "C" fn read(
    p: *mut c_void,
    b: *mut u8,
    n: usize,
    _timeout: u32,
    out: *mut usize,
) -> i32 {
    // SAFETY: test supplies a boxed context and caller-owned temporary buffers.
    let c = unsafe { &mut *p.cast::<Context>() };
    if c.bad_count {
        unsafe { out.write(n + 1) };
        return 0;
    }
    if c.input.is_empty() {
        return 4;
    }
    let len = n.min(c.input.len()).min(3);
    for i in 0..len {
        unsafe { b.add(i).write(c.input.pop_front().unwrap_or(0)) };
    }
    unsafe { out.write(len) };
    0
}
unsafe extern "C" fn write(p: *mut c_void, b: *const u8, n: usize, out: *mut usize) -> i32 {
    // SAFETY: borrowed buffers/context are valid for the callback duration.
    let c = unsafe { &mut *p.cast::<Context>() };
    let bytes = unsafe { std::slice::from_raw_parts(b, n) };
    let packets = match c.decoder.feed(bytes) {
        Ok(v) => v,
        Err(_) => return 5,
    };
    for p in packets {
        let payload = match p.command {
            0x0100 => vec![0, 0xbf, 0x0f, 0x5a, 0xe6, 0x65, 0x9c, 0x40],
            0x0101 => vec![0, 0xc0, 8],
            0x010c => vec![0, 1, 1, 8],
            _ => vec![0],
        };
        match encode(
            &Packet::new(p.command | 0x8000, p.transaction, payload),
            1946,
        ) {
            Ok(v) => c.input.extend(v.concat()),
            Err(_) => return 5,
        }
    }
    unsafe { out.write(n) };
    0
}
unsafe extern "C" fn release(p: *mut c_void) {
    // SAFETY: exactly one release of the Box leased to a successful open.
    let c = unsafe { Box::from_raw(p.cast::<Context>()) };
    c.released.fetch_add(1, Ordering::SeqCst);
}
fn callbacks(bad_count: bool) -> (TransportV1, Arc<AtomicUsize>) {
    let released = Arc::new(AtomicUsize::new(0));
    let c = Context {
        decoder: Decoder::default(),
        input: VecDeque::new(),
        released: released.clone(),
        bad_count,
    };
    (
        TransportV1 {
            struct_size: std::mem::size_of::<TransportV1>() as u32,
            abi_version: 1,
            context: Box::into_raw(Box::new(c)).cast(),
            read: Some(read),
            write: Some(write),
            flush: None,
            release: Some(release),
        },
        released,
    )
}
fn open(bad: bool) -> (u64, Arc<AtomicUsize>) {
    let (t, r) = callbacks(bad);
    let mut h = 0;
    let model = b"realme.buds-air7";
    // SAFETY: all slices/table/output are valid and context remains leased until close.
    assert_eq!(
        unsafe { ob_v1_open(model.as_ptr(), model.len(), &t, &mut h) },
        0
    );
    (h, r)
}
fn call(h: u64, request: &[u8]) -> (i32, BufferV1) {
    let mut b = BufferV1::default();
    // SAFETY: request slice and output are live for this synchronous call.
    let code = unsafe { ob_v1_call(h, request.as_ptr(), request.len(), &mut b) };
    (code, b)
}
#[test]
fn initialized_json_and_typed_paths_share_driver_and_release() {
    let (h, r) = open(false);
    let mut n = Air7NoiseV1::default();
    // SAFETY: writable output; handle is live.
    assert_eq!(unsafe { ob_v1_air7_get_noise(h, &mut n) }, 10);
    assert_eq!(ob_v1_initialize(h), 0);
    assert_eq!(unsafe { ob_v1_air7_get_noise(h, &mut n) }, 0);
    assert_eq!(n.value, 8);
    let (code, mut b) = call(h, br#"{"op":"noise"}"#);
    assert_eq!(code, 0);
    let v: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(b.data, b.len) }).unwrap();
    assert_eq!(v["driver"], "realme.buds-air7");
    assert_eq!(v["data"]["category"], "Anc");
    assert_eq!(unsafe { ob_v1_buffer_release(&mut b) }, 0);
    assert_eq!(ob_v1_close(h), 0);
    assert_eq!(r.load(Ordering::SeqCst), 1);
    assert_eq!(ob_v1_close(h), 3);
}
#[test]
fn unknown_models_and_short_tables_do_not_take_context() {
    let (t, r) = callbacks(false);
    let mut h = 77;
    let model = b"not-an-air7";
    assert_eq!(
        unsafe { ob_v1_open(model.as_ptr(), model.len(), &t, &mut h) },
        2
    );
    assert_eq!(h, 0);
    assert_eq!(r.load(Ordering::SeqCst), 0);
    let mut short = t;
    short.struct_size = 8;
    let model = b"realme.buds-air7";
    assert_eq!(
        unsafe { ob_v1_open(model.as_ptr(), model.len(), &short, &mut h) },
        1
    );
    assert_eq!(r.load(Ordering::SeqCst), 0);
    // SAFETY: construction failed before lease transfer; the test still owns context.
    unsafe { release(t.context) };
    assert_eq!(r.load(Ordering::SeqCst), 1);
}
#[test]
fn corrupt_callback_counts_are_rejected_without_buffer_access() {
    let (h, r) = open(true);
    assert_eq!(ob_v1_initialize(h), 5);
    assert_eq!(ob_v1_close(h), 0);
    assert_eq!(r.load(Ordering::SeqCst), 1);
}
#[test]
fn bad_json_utf8_nulls_and_scalar_ranges_have_statuses() {
    let (h, _) = open(false);
    assert_eq!(ob_v1_initialize(h), 0);
    for request in [
        &b"not json"[..],
        &[255u8][..],
        br#"{"op":"invented"}"#,
        br#"{"op":"set_noise","mode":"Other"}"#,
    ] {
        let (c, b) = call(h, request);
        assert_eq!(c, 1);
        assert_eq!(b.id, 0);
    }
    let mut b = BufferV1::default();
    assert_eq!(unsafe { ob_v1_call(h, std::ptr::null(), 1, &mut b) }, 1);
    assert_eq!(
        unsafe { ob_v1_call(h, std::ptr::null(), usize::MAX, &mut b) },
        1
    );
    assert_eq!(ob_v1_air7_set_noise(h, 999), 1);
    assert_eq!(ob_v1_air7_set_feature(h, 4, 2), 1);
    assert_eq!(ob_v1_set_timeout(h, 0), 1);
    assert_eq!(ob_v1_close(h), 0);
}
#[test]
fn unsafe_sound_and_raw_writes_cannot_bypass_named_guards() {
    let (h, _) = open(false);
    assert_eq!(ob_v1_initialize(h), 0);
    for request in [
        br#"{"op":"find_for","duration_ms":1,"confirm_sound":false}"#.as_slice(),
        br#"{"op":"run_fit_test","timeout_ms":1,"confirm_sound":false}"#,
        br#"{"op":"raw_read","command":1024,"payload":[1]}"#,
    ] {
        assert_eq!(call(h, request).0, 1);
    }
    assert_eq!(ob_v1_close(h), 0);
}
#[test]
fn buffer_ids_are_checked_and_cannot_be_session_handles() {
    let (h, _) = open(false);
    let mut b = BufferV1::default();
    assert_eq!(unsafe { ob_v1_driver_catalog(&mut b) }, 0);
    assert_ne!(b.id, h);
    assert_eq!(ob_v1_close(b.id), 3);
    let mut duplicate = b;
    assert_eq!(unsafe { ob_v1_buffer_release(&mut b) }, 0);
    assert_eq!(b.id, 0);
    assert_eq!(unsafe { ob_v1_buffer_release(&mut duplicate) }, 3);
    assert_eq!(unsafe { ob_v1_buffer_release(&mut b) }, 0);
    assert_eq!(ob_v1_close(h), 0);
}
#[test]
fn errors_are_per_thread_and_include_owned_utf8() {
    assert_eq!(ob_v1_close(0), 3);
    std::thread::spawn(|| {
        assert_eq!(ob_v1_air7_set_noise(0, 999), 1);
        let mut e = ErrorInfo::default();
        let mut b = BufferV1::default();
        assert_eq!(unsafe { ob_v1_last_error(&mut e, &mut b) }, 0);
        assert_eq!(e.status, 1);
        assert!(b.len > 0);
        assert_eq!(unsafe { ob_v1_buffer_release(&mut b) }, 0);
    })
    .join()
    .unwrap();
    let mut e = ErrorInfo::default();
    let mut b = BufferV1::default();
    assert_eq!(unsafe { ob_v1_last_error(&mut e, &mut b) }, 0);
    assert_eq!(e.status, 3);
    assert_eq!(unsafe { ob_v1_buffer_release(&mut b) }, 0);
}
#[test]
fn pod_layout_and_invalid_output_contract_are_explicit() {
    assert_eq!(std::mem::size_of::<Air7NoiseV1>(), 4);
    assert_eq!(std::mem::size_of::<ErrorInfo>(), 16);
    assert_eq!(ob_v1_abi_version(), 1);
    assert_eq!(unsafe { ob_v1_driver_catalog(std::ptr::null_mut()) }, 1);
}
