// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use realme_buds::{
    Client, Error, Transport,
    codec::{Decoder, Packet, encode},
    model::*,
};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    time::Duration,
};
fn hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("fixtures/air7.json")).unwrap()
}
#[test]
fn captures_decode_at_every_stream_chunk_size() {
    for v in fixtures() {
        let f = hex(v["frame"].as_str().unwrap());
        let expected = Packet::new(
            u16::from_str_radix(v["command"].as_str().unwrap(), 16).unwrap(),
            v["transfer"].as_u64().unwrap() as u8,
            hex(v["payload"].as_str().unwrap()),
        );
        for n in 1..=f.len() {
            let mut d = Decoder::default();
            let mut packets = Vec::new();
            for b in f.chunks(n) {
                packets.extend(d.feed(b).unwrap());
            }
            assert_eq!(
                packets,
                vec![expected.clone()],
                "chunk size {n}, command {}",
                expected.command
            );
        }
        if expected.command != 0x841b {
            assert_eq!(encode(&expected, 1946).unwrap(), vec![f]);
        }
    }
}
fn payload(command: &str) -> Vec<u8> {
    hex(
        fixtures().iter().find(|p| p["command"] == command).unwrap()["payload"]
            .as_str()
            .unwrap(),
    )
}
#[test]
fn reads_match_observed_screens() {
    let b = parse_battery(&payload("8106")[1..]).unwrap();
    assert_eq!(
        b.iter().map(|b| b.percent).collect::<Vec<_>>(),
        vec![Some(100), Some(100), Some(10)]
    );
    let eq = parse_custom_eq(&payload("8122")[1..]).unwrap();
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0].id, 4);
    assert_eq!(eq[0].name, "TestPreset");
    assert_eq!(
        eq[0].bands.iter().map(|b| b.frequency).collect::<Vec<_>>(),
        vec![62, 250, 1000, 4000, 8000, 16000]
    );
    assert_eq!(
        eq[0].bands.iter().map(|b| b.gain).collect::<Vec<_>>(),
        vec![-6, 0, 2, 0, -3, -6]
    );
    let f = parse_features(&payload("810d")[1..]).unwrap();
    assert_eq!(
        f,
        vec![FeatureState {
            id: 6,
            enabled: false
        }]
    );
    assert_eq!(
        parse_noise(&payload("810c")[1..]).unwrap(),
        NoiseState {
            action: 1,
            kind: 1,
            value: 1
        }
    );
    assert_eq!(parse_touch(&payload("8108")[1..]).unwrap().len(), 10);
    assert!(parse_wear(&payload("8109")[1..]).unwrap()[0].in_case);
    assert_eq!(parse_dynamic_bands(&payload("812c")[1..]).unwrap().len(), 3);
}
#[test]
fn eq_payload_actions_signed_gains_and_utf8() {
    let eq = CustomEq::air7(4, "音", [-6, -3, 0, 2, 3, 6]);
    let p = eq.set_payload(EqAction::Update).unwrap();
    assert_eq!(&p[..5], &[2, 250, 6, 4, 3]);
    assert_eq!(&p[5..8], "音".as_bytes());
    assert_eq!(&p[8..12], &[6, 62, 0, 250]);
    assert_eq!(eq.set_payload(EqAction::Add).unwrap()[0], 1);
    assert_eq!(eq.set_payload(EqAction::Delete).unwrap()[0], 3);
    let mut bad = eq.clone();
    bad.bands[0].gain = -7;
    assert!(bad.validate().is_err());
    bad = eq.clone();
    bad.name = "x".repeat(256);
    assert!(bad.validate().is_err());
}
#[test]
fn capability_bitmap_and_mandatory_notification_query() {
    let c = Capabilities::parse(&hex("bf0f5ae6659c40"));
    for id in [
        0x0100, 0x0106, 0x0122, 0x012c, 0x0200, 0x0205, 0x0418, 0x041b,
    ] {
        assert!(c.supports(id));
    }
    assert!(!c.supports(0x042c));
}
#[test]
fn fragment_and_long_length_roundtrip() {
    for len in [0, 1, 120, 127, 128, 500, 2000, 65535] {
        let p = Packet::new(0x1234, 255, vec![0xaa; len]);
        for max in [512, 1946] {
            let frames = encode(&p, max).unwrap();
            assert!(frames.iter().all(|f| f.len() <= max));
            let mut d = Decoder::default();
            let mut out = Vec::new();
            for f in frames {
                for b in f.chunks(7) {
                    out.extend(d.feed(b).unwrap());
                }
            }
            assert_eq!(out, vec![p.clone()]);
        }
    }
}
#[test]
fn coalesced_frames_and_split_header() {
    let a = Packet::new(0x8106, 1, vec![0, 0]);
    let b = Packet::new(0x8403, 2, vec![0]);
    let mut s = encode(&a, 1946).unwrap().remove(0);
    s.extend(encode(&b, 1946).unwrap().remove(0));
    let mut d = Decoder::default();
    assert!(d.feed(&s[..1]).unwrap().is_empty());
    assert_eq!(d.feed(&s[1..]).unwrap(), vec![a, b]);
}
#[test]
fn known_dynamic_ack_quirk_is_narrow() {
    let p = Packet::parse(&hex("1b840301000001")).unwrap();
    assert_eq!(p.payload, vec![0, 1]);
    assert!(Packet::parse(&hex("03840301000001")).is_err());
    assert!(Packet::parse(&hex("1b840301000004")).is_err());
    assert!(Packet::parse(&hex("1b840301000101")).is_err());
}
#[test]
fn reject_truncated_and_invalid_data_without_panics() {
    for data in [payload("8106")[1..].to_vec(), payload("8122")[1..].to_vec()] {
        for n in 0..data.len() {
            if data.len() > 10 {
                assert!(parse_custom_eq(&data[..n]).is_err());
            } else {
                assert!(parse_battery(&data[..n]).is_err());
            }
        }
    }
    for b in [
        &[0xaa, 0x80, 0x80, 0x80][..],
        &[0xaa, 0x00][..],
        &[0xff][..],
        &[0xaa, 3, 2, 0, 1][..],
    ] {
        assert!(Decoder::default().feed(b).is_err());
    }
    assert_eq!(parse_battery(&[1, 1, 127]).unwrap()[0].percent, None);
    assert!(parse_features(&[1, 4, 2]).is_err());
    assert!(parse_noise(&[]).is_err());
}
#[test]
fn malformed_corpus_does_not_panic() {
    let mut x = 123456u32;
    for len in 0..300 {
        let bytes: Vec<_> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        let _ = Decoder::default().feed(&bytes);
        let _ = parse_custom_eq(&bytes);
        let _ = parse_multipoint(&bytes);
        let _ = parse_battery(&bytes);
    }
}
#[test]
fn multipoint_skips_extensions_and_nul_terminated_names() {
    let mut b = vec![1, 6, 5, 4, 3, 2, 1, 10, 2, 1, 5];
    b.extend(b"Test\0");
    b.extend([99, 100]);
    let p = parse_multipoint(&b).unwrap();
    assert_eq!(p[0].address, "01:02:03:04:05:06");
    assert_eq!(p[0].name, "Test");
    assert!(p[0].current);
    assert_eq!(p[0].flags, 1);
    assert_eq!(p[0].extension, vec![99, 100]);
}

struct Scripted {
    input: VecDeque<u8>,
    written: Vec<u8>,
    chunk: usize,
}
impl Scripted {
    fn new(packets: Vec<Packet>) -> Self {
        Self {
            input: packets
                .iter()
                .flat_map(|p| encode(p, 1946).unwrap().concat())
                .collect(),
            written: Vec::new(),
            chunk: 3,
        }
    }
}
impl Read for Scripted {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if self.input.is_empty() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "mock deadline"));
        }
        let n = b.len().min(self.chunk).min(self.input.len());
        for v in &mut b[..n] {
            *v = self.input.pop_front().unwrap();
        }
        Ok(n)
    }
}
impl Write for Scripted {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = b.len().min(2);
        self.written.extend(&b[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Transport for Scripted {
    fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn transaction_matching_partial_writes_and_notifications() {
    let mock = Scripted::new(vec![
        Packet::new(0x0204, 12, hex("010301640264030a")),
        Packet::new(0x8106, 9, vec![0, 0]),
        Packet::new(0x8106, 0, hex("000301640264030a")),
    ]);
    let mut c = Client::new(mock);
    assert_eq!(c.battery().unwrap().len(), 3);
    assert!(matches!(
        c.next_event(Duration::from_secs(1)).unwrap(),
        Event::Battery(_)
    ));
    assert!(matches!(
        c.next_event(Duration::from_secs(1)).unwrap(),
        Event::Raw(_)
    ));
    assert_eq!(
        Decoder::default()
            .feed(&c.into_transport().written)
            .unwrap(),
        vec![Packet::new(0x0106, 0, vec![])]
    );
}
#[test]
fn status_rejection_and_no_automatic_retry() {
    let mut c = Client::new(Scripted::new(vec![Packet::new(0x8404, 0, vec![14])]));
    assert!(matches!(
        c.set_noise(NoiseMode::Maximum),
        Err(Error::Rejected { status: 14, .. })
    ));
    let mut c = Client::new(Scripted::new(vec![]));
    assert!(matches!(c.battery(), Err(Error::Timeout)));
    let t = c.into_transport();
    assert_eq!(Decoder::default().feed(&t.written).unwrap().len(), 1);
}
#[test]
fn transfer_id_wraps() {
    let p = (0..257)
        .map(|i| Packet::new(0x8106, i as u8, vec![0, 0]))
        .collect();
    let mut c = Client::new(Scripted::new(p));
    for _ in 0..257 {
        assert!(c.battery().unwrap().is_empty());
    }
    let p = Decoder::default()
        .feed(&c.into_transport().written)
        .unwrap();
    assert_eq!(p[255].transaction, 255);
    assert_eq!(p[256].transaction, 0);
}
#[test]
fn capability_gating_prevents_writes() {
    let mock = Scripted::new(vec![
        Packet::new(0x8100, 0, vec![0, 0]),
        Packet::new(0x8101, 1, vec![0, 0xc0, 8]),
    ]);
    let mut c = Client::new(mock);
    c.initialize().unwrap();
    assert!(matches!(
        c.set_feature(Feature::DynamicAudio, true),
        Err(Error::Unsupported(0x0403))
    ));
    assert_eq!(
        Decoder::default()
            .feed(&c.into_transport().written)
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn dynamic_band_can_confirm_by_notification() {
    let band = DynamicBand {
        min: -5,
        max: 5,
        value: -5,
        kind: 1,
    };
    let mut c = Client::new(Scripted::new(vec![Packet::new(
        0x0508,
        199,
        hex("fb05fb01"),
    )]));
    c.set_dynamic_band(band).unwrap();
    assert_eq!(
        c.next_event(Duration::from_secs(1)).unwrap(),
        Event::DynamicBand(band)
    );
}
#[test]
fn invalid_settings_do_not_touch_transport() {
    let mut c = Client::new(Scripted::new(vec![]));
    assert!(c.set_eq_preset(5).is_err());
    assert!(c.set_touch(&[]).is_err());
    assert!(c.set_timeout(Duration::ZERO).is_err());
    assert!(c.into_transport().written.is_empty());
}

#[test]
fn air7_extended_cycle_notification_does_not_abort_acknowledgement() {
    let push = Packet::new(0x0204, 135, hex("0302010300"));
    let mut c = Client::new(Scripted::new(vec![push, Packet::new(0x8404, 0, vec![0])]));
    c.set_noise_cycle(NoiseCycle::OffTransparency).unwrap();
    assert_eq!(
        c.next_event(Duration::from_secs(1)).unwrap(),
        Event::Noise(NoiseState {
            action: 2,
            kind: 1,
            value: 3
        })
    );
}

#[test]
fn unknown_noise_extension_is_retained_raw() {
    let p = Packet::new(0x0204, 1, hex("0302010301"));
    assert_eq!(parse_event(p.clone()).unwrap(), Event::Raw(p));
}

#[test]
fn wearing_scenarios_and_missing_or_offline_ears_are_explicit() {
    for (l, r, expected) in [
        (1, 1, WearingScenario::Neither),
        (3, 1, WearingScenario::LeftOnly),
        (1, 3, WearingScenario::RightOnly),
        (3, 3, WearingScenario::Both),
    ] {
        assert_eq!(
            wearing_summary(&parse_wear(&[2, 1, l, 2, r]).unwrap()).scenario,
            expected
        );
    }
    let partial = wearing_summary(&parse_wear(&[1, 2, 3]).unwrap());
    assert!(partial.left.is_none());
    assert!(partial.right.unwrap().worn);
    assert_eq!(partial.scenario, WearingScenario::Unknown);
    assert_eq!(
        wearing_summary(&parse_wear(&[2, 1, 5, 2, 3]).unwrap()).scenario,
        WearingScenario::Unknown
    );
    assert_eq!(wearing_summary(&[]).scenario, WearingScenario::Unknown);
}

#[test]
fn unavailable_battery_and_charging_are_preserved() {
    let b = parse_battery(&[2, 1, 0xff, 2, 0xe4]).unwrap();
    assert_eq!(b[0].percent, None);
    assert!(b[0].charging);
    assert_eq!(b[0].raw, 255);
    assert_eq!(b[1].percent, Some(100));
    assert!(b[1].charging);
    assert!(parse_battery(&[2, 1, 100]).is_err());
}

#[test]
fn remaining_dynamic_band_ack_metadata_is_not_a_band_id() {
    let mock = Scripted::new(vec![Packet::new(0x841b, 0, vec![0, 1])]);
    Client::new(mock)
        .set_dynamic_band(DynamicBand {
            min: -5,
            max: 5,
            value: 5,
            kind: 3,
        })
        .unwrap();
}

#[test]
fn diagnostic_reply_length_quirks_are_narrow() {
    let lang = hex("aa09000020810d00010003");
    assert_eq!(
        Decoder::default().feed(&lang).unwrap()[0].payload,
        vec![0, 3]
    );
    let status = hex("aa0e00002981b2050000010112569457");
    let p = Decoder::default().feed(&status).unwrap().remove(0);
    assert_eq!(p.payload, hex("00010112569457"));
    let detect = hex("aa0d00002a81b705000007010002ce");
    assert!(matches!(
        parse_event(Decoder::default().feed(&detect).unwrap().remove(0)).unwrap(),
        Event::FeatureDetection(_)
    ));
    assert!(Packet::parse(&hex("21810d00010003")).is_err());
    assert!(Packet::parse(&hex("2981b2050000020112569457")).is_err());
}

#[test]
fn multi_reply_status_query_uses_device_event_counter_and_keeps_unrelated_events() {
    let m = Scripted::new(vec![
        Packet::new(0x8129, 178, hex("00010112569457")),
        Packet::new(0x0204, 33, hex("020201010203")),
        Packet::new(0x8129, 179, hex("00010220304050")),
    ]);
    let mut c = Client::new(m);
    let data = c.status_features(&[1, 2, 3]).unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(data[0].id, 1);
    assert_eq!(data[1].id, 2);
    assert!(matches!(
        c.next_event(Duration::from_secs(1)).unwrap(),
        Event::Wear(_)
    ));
}

#[test]
fn under_the_hood_parsers_handle_zero_and_truncated_counts() {
    use realme_buds::advanced::*;
    assert!(parse_status_features(&[0]).unwrap().is_empty());
    assert!(parse_stored_data(&[0]).unwrap().is_empty());
    assert!(parse_status_features(&[1, 2, 0]).is_err());
    assert!(parse_stored_data(&[1, 1, 255, 255]).is_err());
    let ota = parse_upgrade_capabilities(&[1, 1, 3]).unwrap();
    assert!(ota[0].supported);
    assert_eq!(ota[0].upgrade_type, 1);
    assert_eq!(
        parse_stored_data(&[1, 4, 2, 0, 5, 6]).unwrap()[0].data,
        vec![5, 6]
    );
    for &p in ReadProperty::ALL {
        assert!((0x0100..=0x012c).contains(&p.command()));
    }
}

#[test]
fn user_interaction_and_host_requests_are_exposed() {
    let p = Packet::new(0x0204, 1, hex("f10201021a010500"));
    let Event::UserInteraction(e) = parse_event(p).unwrap() else {
        panic!("wrong event")
    };
    assert_eq!(e.component, 2);
    assert_eq!(e.function, 26);
    assert_eq!(e.options, vec![5]);
    assert_eq!(
        parse_event(Packet::new(0x0502, 1, vec![5])).unwrap(),
        Event::HostOperation(5)
    );
}

#[test]
fn finding_is_blocked_for_one_worn_or_missing_ear() {
    for b in [hex("000201010203"), hex("00010201")] {
        let mut c = Client::new(Scripted::new(vec![Packet::new(0x8109, 0, b)]));
        assert!(matches!(c.find_buds(true), Err(Error::Invalid(_))));
        let commands = Decoder::default()
            .feed(&c.into_transport().written)
            .unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, 0x0109);
    }
}

#[test]
fn fit_rejection_still_sends_stop() {
    // wear, notification capabilities, start rejection, stop acknowledgement.
    let mut c = Client::new(Scripted::new(vec![
        Packet::new(0x8109, 0, hex("000201010203")),
        Packet::new(0x8200, 1, vec![0, 2, 1, 2]),
        Packet::new(0x8405, 3, vec![14]),
        Packet::new(0x8405, 4, vec![0]),
    ]));
    assert!(matches!(
        c.run_fit_test_with_cancel(Duration::from_secs(1), || false),
        Err(Error::Rejected { status: 14, .. })
    ));
    let packets = Decoder::default()
        .feed(&c.into_transport().written)
        .unwrap();
    assert_eq!(packets.last().unwrap().payload, vec![0]);
    assert_eq!(packets.last().unwrap().command, 0x0405);
}

#[test]
fn fit_results_and_cancelled_finding_have_cleanup() {
    let mut c = Client::new(Scripted::new(vec![
        Packet::new(0x8109, 0, hex("000201030203")),
        Packet::new(0x8200, 1, vec![0, 2, 1, 2]),
        Packet::new(0x8405, 3, vec![0]),
        Packet::new(0x0204, 34, hex("0401020203")),
        Packet::new(0x8405, 4, vec![0]),
    ]));
    let f = c
        .run_fit_test_with_cancel(Duration::from_secs(1), || false)
        .unwrap();
    assert_eq!(f.len(), 2);
    assert_eq!(f[1].ear, 2);
    let mut c = Client::new(Scripted::new(vec![Packet::new(0x8400, 0, vec![0])]));
    assert!(matches!(
        c.find_buds_for_with_cancel(Duration::from_secs(1), || true),
        Err(Error::Cancelled)
    ));
    let p = Decoder::default()
        .feed(&c.into_transport().written)
        .unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].payload, vec![0]);
}

#[test]
fn captured_both_worn_fit_verdicts_are_good_not_progress() {
    let packet = Decoder::default()
        .feed(&hex("aa0c00000402dd05000401010201"))
        .unwrap()
        .pop()
        .unwrap();
    let Event::Fit(f) = parse_event(packet).unwrap() else {
        panic!("expected fit results")
    };
    assert_eq!(
        f.iter().map(|v| (v.ear, v.outcome())).collect::<Vec<_>>(),
        vec![(1, FitOutcome::GoodFit), (2, FitOutcome::GoodFit)]
    );
    assert_eq!(
        FitResult { ear: 1, result: 0 }.outcome(),
        FitOutcome::GeneralFit
    );
    assert_eq!(
        FitResult { ear: 1, result: 3 }.outcome(),
        FitOutcome::DetectionFailed
    );
    assert_eq!(
        FitResult { ear: 1, result: 4 }.outcome(),
        FitOutcome::InterruptedByWear
    );
    assert_eq!(
        FitResult { ear: 1, result: 5 }.outcome(),
        FitOutcome::InterruptedByApp
    );
    assert_eq!(
        FitResult { ear: 1, result: 99 }.outcome(),
        FitOutcome::Unknown(99)
    );
}

#[test]
fn fit_result_before_start_ack_is_not_lost() {
    let mut c = Client::new(Scripted::new(vec![
        Packet::new(0x8109, 0, hex("000201030203")),
        Packet::new(0x8200, 1, vec![0, 2, 1, 2]),
        Packet::new(0x0204, 34, hex("0401020203")),
        Packet::new(0x8405, 3, vec![0]),
        Packet::new(0x8405, 4, vec![0]),
    ]));
    assert_eq!(
        c.run_fit_test_with_cancel(Duration::from_secs(1), || false)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fit_start_wait_is_cancellable_before_acknowledgement() {
    let mut c = Client::new(Scripted::new(vec![
        Packet::new(0x8109, 0, hex("000201030203")),
        Packet::new(0x8200, 1, vec![0, 2, 1, 2]),
        Packet::new(0x8405, 4, vec![0]),
    ]));
    let mut polls = 0;
    assert!(matches!(
        c.run_fit_test_with_cancel(Duration::from_secs(1), || {
            polls += 1;
            polls == 2
        }),
        Err(Error::Cancelled)
    ));
    let sent = Decoder::default()
        .feed(&c.into_transport().written)
        .unwrap();
    assert_eq!(
        sent.iter()
            .filter(|p| p.command == realme_buds::commands::FIT_TEST)
            .map(|p| p.payload.clone())
            .collect::<Vec<_>>(),
        vec![vec![1], vec![0]]
    );
}

#[test]
fn captured_left_worn_right_in_case_keeps_absent_battery_distinct() {
    let s = Snapshot {
        wear: parse_wear(&[3, 1, 3, 2, 0, 3, 4]).unwrap(),
        battery: parse_battery(&[2, 1, 100, 3, 0]).unwrap(),
        ..Default::default()
    };
    assert_eq!(s.wearing_summary().scenario, WearingScenario::LeftOnly);
    assert!(s.wearing_summary().right.unwrap().in_case);
    assert!(s.battery_for(Component::Right).is_none());
    assert_eq!(s.battery_for(Component::Case).unwrap().percent, Some(0));
    assert_eq!(s.wear[2].raw, 4); // case-link flags are not proof of lid position or battery freshness
}

#[test]
fn tracing_records_actual_partial_bytes_without_losing_stream_semantics() {
    let mock = Scripted::new(vec![Packet::new(0x8106, 0, vec![0, 0])]);
    let log = std::io::Cursor::new(Vec::new());
    let mut c = Client::new(realme_buds::transport::trace::Traced::new(mock, log));
    assert!(c.battery().unwrap().is_empty());
    assert_eq!(
        Decoder::default()
            .feed(&c.into_transport().into_inner().written)
            .unwrap()
            .len(),
        1
    );
}
