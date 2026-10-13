// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Wire command IDs, derived from captures and cross-checked against the Android SDK.
pub const CAPABILITIES: u16 = 0x0100;
pub const SET_HOST_MTU: u16 = 0x0101;
pub const SET_HOST_FRAME_SIZE: u16 = 0x0102;
pub const PRODUCT_ID: u16 = 0x0103;
#[deprecated(note = "0x0103 returns the product ID, not a peer frame size")]
pub const PEER_FRAME_SIZE: u16 = PRODUCT_ID;
pub const VERSIONS: u16 = 0x0105;
pub const BATTERY: u16 = 0x0106;
pub const TOUCH: u16 = 0x0108;
pub const WEAR: u16 = 0x0109;
pub const NOISE: u16 = 0x010c;
pub const FEATURES: u16 = 0x010d;
pub const EQ_PRESET: u16 = 0x010f;
pub const VOLUME_ENHANCER: u16 = 0x0110;
pub const MULTIPOINT: u16 = 0x0112;
pub const CUSTOM_EQ: u16 = 0x0122;
pub const DYNAMIC_BANDS: u16 = 0x012c;
pub const NOTIFICATION_CAPABILITIES: u16 = 0x0200;
pub const NOTIFICATION: u16 = 0x0204;
pub const SUBSCRIBE: u16 = 0x0205;
pub const FIND_BUDS: u16 = 0x0400;
pub const SET_TOUCH: u16 = 0x0401;
pub const SET_FEATURE: u16 = 0x0403;
pub const SET_NOISE: u16 = 0x0404;
pub const FIT_TEST: u16 = 0x0405;
pub const SET_EQ_PRESET: u16 = 0x0406;
pub const SET_VOLUME_ENHANCER: u16 = 0x0407;
pub const MULTIPOINT_ACTION: u16 = 0x040b;
pub const SET_CUSTOM_EQ: u16 = 0x0418;
pub const SET_DYNAMIC_BAND: u16 = 0x041b;
pub const CUSTOM_EQ_EVENT: u16 = 0x0506;
pub const DYNAMIC_BAND_EVENT: u16 = 0x0508;

// Capability bit index -> command IDs. Empty entries are reserved by the vendor.
// A set bit means advertised support, not proof that every operation was tested.
pub const CAPABILITY_COMMANDS: [&[u16]; 60] = [
    &[0x0105],
    &[0x0106],
    &[0x0107],
    &[0x0108, 0x0401],
    &[0x0109],
    &[0x0400],
    &[0x0402],
    &[0x010d, 0x0403],
    &[0x010c, 0x0404],
    &[0x0405],
    &[0x0406, 0x010f],
    &[0x0110, 0x0407],
    &[],
    &[0x0408],
    &[0x0409],
    &[0x040a, 0x0111],
    &[],
    &[],
    &[],
    &[0x040e, 0x040d, 0x0115, 0x0116],
    &[],
    &[],
    &[0x0205],
    &[0x0f00],
    &[],
    &[0x0118, 0x0411],
    &[0x011a, 0x0412],
    &[],
    &[],
    &[0x0112, 0x040b],
    &[0x011e, 0x011f, 0x0415],
    &[],
    &[0x0120],
    &[],
    &[0x0122, 0x0418],
    &[],
    &[],
    &[],
    &[0x0124, 0x041b],
    &[],
    &[],
    &[],
    &[],
    &[0x041e],
    &[],
    &[],
    &[0x0128, 0x050e],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[0x012c],
    &[],
    &[],
    &[],
    &[0x0131, 0x0428],
    &[],
];
pub const FEATURE_IDS: &[u8] = &[4, 5, 6, 7, 8, 9, 11, 12, 17, 19, 24, 26, 27, 29, 38, 54];
pub const TOUCH_FUNCTION_IDS: &[u8] = &[1, 3, 4, 5, 6, 11, 12, 8, 13, 17, 31, 26, 27, 0];
