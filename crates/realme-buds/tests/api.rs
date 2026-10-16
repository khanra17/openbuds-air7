// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
#![cfg(feature = "json")]
use realme_buds::devices::air7::api::Request;

#[test]
fn json_requests_reject_unknown_operations_fields_and_bad_identifiers() {
    for request in [
        r#"{"op":"invented"}"#,
        r#"{"op":"noise","typo":true}"#,
        r#"{"op":"set_noise","mode":"Maximum","typo":true}"#,
        r#"{"op":"set_noise","mode":"NotAMode"}"#,
        r#"{"op":"set_feature","feature":"Unknown","enabled":true}"#,
    ] {
        assert!(
            serde_json::from_str::<Request>(request).is_err(),
            "accepted {request}"
        );
    }
}
#[test]
fn safety_confirmations_are_required_not_implicit_defaults() {
    for request in [
        r#"{"op":"find_for","duration_ms":1000}"#,
        r#"{"op":"run_fit_test","timeout_ms":1000}"#,
        r#"{"op":"set_volume_enhancer","enabled":true}"#,
        r#"{"op":"set_feature_id","id":99,"enabled":true}"#,
    ] {
        assert!(
            serde_json::from_str::<Request>(request).is_err(),
            "accepted {request}"
        );
    }
}
#[test]
fn handshake_is_explicit_and_other_requests_are_not_handshakes() {
    assert!(
        serde_json::from_str::<Request>(r#"{"op":"initialize"}"#)
            .unwrap()
            .is_initialize()
    );
    assert!(
        !serde_json::from_str::<Request>(r#"{"op":"noise"}"#)
            .unwrap()
            .is_initialize()
    );
}
