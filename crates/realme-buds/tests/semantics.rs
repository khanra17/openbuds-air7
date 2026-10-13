// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use realme_buds::*;
use std::{collections::VecDeque, io, time::Duration};
#[derive(Default)]
struct Wire {
    input: VecDeque<u8>,
    written: Vec<u8>,
}
impl Wire {
    fn ack() -> Self {
        Self {
            input: codec::encode(&codec::Packet::new(0x8404, 0, vec![0]), 1946)
                .unwrap()
                .concat()
                .into(),
            written: vec![],
        }
    }
}
impl io::Read for Wire {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = b.len().min(self.input.len());
        if n == 0 {
            return Err(io::ErrorKind::TimedOut.into());
        }
        for v in b.iter_mut().take(n) {
            *v = self.input.pop_front().unwrap();
        }
        Ok(n)
    }
}
impl io::Write for Wire {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.written.extend(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Transport for Wire {
    fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn all_current_noise_values_and_noncurrent_channels_stay_distinct() {
    for action in [1, 2, 4, 255] {
        for kind in [1, 2, 255] {
            for value in 0..=255 {
                let n = NoiseState {
                    action,
                    kind,
                    value,
                };
                let d = n.describe();
                let expected = if action != 1 || kind != 1 {
                    None
                } else {
                    match value {
                        1 => Some(NoiseCategory::Off),
                        2 => Some(NoiseCategory::Transparency),
                        4 | 8 | 16 | 32 => Some(NoiseCategory::Anc),
                        _ => None,
                    }
                };
                assert_eq!(d.category, expected);
                assert_eq!(d.anc_enabled, expected.map(|v| v == NoiseCategory::Anc));
                assert_eq!(d.raw, n);
                if expected != Some(NoiseCategory::Anc) {
                    assert_eq!(d.active_anc_level, None);
                }
            }
        }
    }
}
#[test]
fn anc_toggle_uses_same_mode_opcode_and_explicit_level() {
    for level in AncLevel::ALL {
        let mut c = Client::new(Wire::ack());
        c.set_anc_enabled(true, Some(level)).unwrap();
        let p = codec::Decoder::default()
            .feed(&c.into_transport().written)
            .unwrap();
        assert_eq!(p[0].command, commands::SET_NOISE);
        assert_eq!(p[0].payload, vec![1, 1, level.mode() as u8]);
    }
    let mut c = Client::new(Wire::default());
    assert!(c.set_anc_enabled(true, None).is_err());
    assert!(c.into_transport().written.is_empty());
    let mut c = Client::new(Wire::ack());
    c.set_anc_enabled(false, None).unwrap();
    assert_eq!(
        codec::Decoder::default()
            .feed(&c.into_transport().written)
            .unwrap()[0]
            .payload,
        vec![1, 1, 1]
    );
}
#[test]
fn transparency_is_exclusive_not_an_independent_flag() {
    for (enable, value) in [(true, 2), (false, 1)] {
        let mut c = Client::new(Wire::ack());
        c.set_transparency_enabled(enable).unwrap();
        assert_eq!(
            codec::Decoder::default()
                .feed(&c.into_transport().written)
                .unwrap()[0]
                .payload,
            vec![1, 1, value]
        );
    }
}
#[test]
fn host_preference_is_not_inferred_from_off_or_adaptive_depth() {
    let mut p = AncPreference::default();
    p.observe(NoiseState {
        action: 1,
        kind: 1,
        value: 1,
    });
    assert_eq!(p.preferred_level, None);
    p.observe(NoiseState {
        action: 4,
        kind: 1,
        value: 16,
    });
    assert_eq!(p.preferred_level, None);
    p.choose(AncLevel::Mild);
    p.observe(NoiseState {
        action: 1,
        kind: 1,
        value: 2,
    });
    assert_eq!(p.preferred_level, Some(AncLevel::Mild));
    p.observe(NoiseState {
        action: 1,
        kind: 1,
        value: 32,
    });
    assert_eq!(p.preferred_level, Some(AncLevel::Adaptive));
}
#[test]
fn names_cover_real_components_and_preserve_unknown_ids() {
    assert_eq!(Component::describe(1).name, "Left earbud");
    assert_eq!(Component::describe(2).name, "Right earbud");
    assert_eq!(Component::describe(3).name, "Charging case");
    let u = Component::describe(99);
    assert!(!u.identified);
    assert_eq!(u.id, 99);
    assert!(u.name.contains("99"));
    assert!(!u.name.contains("Right"));
}
#[test]
fn missing_battery_is_not_zero_or_not_charging() {
    let v = describe_battery(&[Battery {
        component: 1,
        percent: Some(0),
        charging: false,
        raw: 0,
    }]);
    assert_eq!(v[0].percent, Some(0));
    assert_eq!(v[0].availability, Availability::Reported);
    assert_eq!(v[1].percent, None);
    assert_eq!(v[1].charging_flag, None);
    assert_eq!(v[1].availability, Availability::NotReported);
    let v = describe_battery(&[Battery {
        component: 2,
        percent: None,
        charging: true,
        raw: 255,
    }]);
    assert_eq!(v[1].raw.unwrap().raw, 255);
    assert_eq!(v[1].percent, None);
}
#[test]
fn case_flags_do_not_become_lid_or_wearing_measurements() {
    let v = describe_wear(&[WearState {
        component: 3,
        in_case: true,
        worn: false,
        connected: false,
        raw: 4,
    }]);
    assert_eq!(v[2].in_case, None);
    assert_eq!(v[2].worn, None);
    assert_eq!(v[2].connection_flag, Some(false));
    assert_eq!(v[2].raw.unwrap().raw, 4);
}
#[test]
fn missing_features_remain_unknown_and_only_requested_ids_are_described() {
    let v = describe_feature_ids(&[], &[6, 99]);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].enabled, None);
    assert_eq!(v[1].feature.id, 99);
    assert!(!v[1].feature.identified);
    let v = describe_features(&[FeatureState {
        id: 4,
        enabled: false,
    }]);
    assert_eq!(
        v.iter().find(|v| v.feature.id == 4).unwrap().enabled,
        Some(false)
    );
    assert_eq!(v.iter().find(|v| v.feature.id == 6).unwrap().enabled, None);
}
#[test]
fn software_selected_eq_is_not_the_stock_preset_and_conflicts_are_retained() {
    let mut s = Snapshot {
        eq_preset: Some(0),
        ..Default::default()
    };
    let mut e = CustomEq::air7(4, "Original user EQ", [0; 6]);
    e.selected = true;
    s.custom_eq.push(e.clone());
    assert!(matches!(
        s.sound_selection(),
        SoundSelection::Custom { id: 4, .. }
    ));
    e.id = 5;
    s.custom_eq.push(e);
    assert_eq!(
        s.sound_selection(),
        SoundSelection::ConflictingCustomSelections(vec![4, 5])
    );
}
#[test]
fn touch_labels_follow_both_sdk_converter_directions() {
    // ItemFactory.f: next=2/prev=3/up=8/down=9; ParamsConverter.A maps 2->6,3->5,8->11,9->12.
    assert_eq!(TouchFunction::NextTrack as u8, 6);
    assert_eq!(TouchFunction::PreviousTrack as u8, 5);
    assert_eq!(TouchFunction::VolumeUp as u8, 11);
    assert_eq!(TouchFunction::VolumeDown as u8, 12);
    assert_eq!(touch_function_identifier(6).name, "Next track");
    assert_eq!(touch_function_identifier(5).name, "Previous track");
    assert_eq!(touch_function_identifier(11).name, "Volume up");
    assert_eq!(touch_function_identifier(12).name, "Volume down");
    assert!(!touch_function_identifier(26).name.contains("Find phone"));
    let v = describe_touch(&[
        TouchBinding {
            ear: 1,
            button: 1,
            gesture: 1,
            function: 0,
        },
        TouchBinding {
            ear: 2,
            button: 1,
            gesture: 6,
            function: 0,
        },
    ]);
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].gesture.name, "Long hold");
}
#[test]
fn failed_reads_are_not_successful_empty_state() {
    let s = Snapshot {
        errors: vec![
            "battery: timeout".into(),
            "wear: malformed".into(),
            "features: timeout".into(),
        ],
        ..Default::default()
    };
    let d = s.describe();
    assert!(
        d.battery
            .iter()
            .all(|v| v.availability == Availability::ReadFailed)
    );
    assert!(d.wear.iter().all(|v| v.worn.is_none()));
    assert!(d.features.iter().all(|v| v.enabled.is_none()));
    assert_eq!(d.raw.errors, s.errors);
}
#[test]
fn named_json_keeps_raw_identifiers_and_unavailable_values() {
    let d = Snapshot::default().describe();
    let j = serde_json::to_value(d).unwrap();
    assert_eq!(j["battery"][1]["component"]["name"], "Right earbud");
    assert!(j["battery"][1]["percent"].is_null());
    assert!(j.get("raw").is_some());
}
#[test]
fn unknown_multipoint_and_band_values_are_never_given_known_names() {
    assert!(!multipoint_state_identifier(99).identified);
    assert!(!band_identifier(4).identified);
    assert!(!preset_identifier(99).identified);
    assert!(gesture_identifier(99).name.contains("99"));
}
#[test]
fn adaptive_report_does_not_assert_active_anc_or_a_stored_preference() {
    let r = advanced::PropertyResult {
        property: advanced::ReadProperty::NoiseAdaptiveLevel,
        command: commands::NOISE,
        advertised: true,
        data: Some(vec![4, 1, 16]),
        error: None,
    };
    assert!(matches!(
        r.describe().value,
        Some(DiagnosticValue::AdaptiveLevel {
            reported_level: Some(AncLevel::Moderate),
            ..
        })
    ));
    let r = advanced::PropertyResult {
        data: None,
        error: Some("timeout".into()),
        ..r
    };
    assert!(r.describe().value.is_none());
}
