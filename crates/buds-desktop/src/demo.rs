// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Deterministic synthetic state for offline previews and documentation.
//! No captured addresses, host names, user presets or live measurements are used.
use realme_buds::*;

pub(crate) fn snapshot() -> Snapshot {
    Snapshot {
        capabilities: Capabilities::parse(&[0xbf, 0x0f, 0x5a, 0xe6, 0x65, 0x9c, 0x40]),
        battery: [(1, 84), (2, 81), (3, 65)]
            .into_iter()
            .map(|(component, percent)| Battery {
                component,
                percent: Some(percent),
                charging: false,
                raw: percent,
            })
            .collect(),
        wear: [(1, 3), (2, 3), (3, 4)]
            .into_iter()
            .map(|(component, raw)| WearState {
                component,
                in_case: raw & 1 == 0,
                worn: raw & 2 != 0,
                connected: raw & 4 == 0,
                raw,
            })
            .collect(),
        features: Feature::AIR7
            .iter()
            .map(|(f, _)| FeatureState {
                id: *f as u8,
                enabled: matches!(
                    f,
                    Feature::AutoPlayPause | Feature::DualDevice | Feature::WindNoiseReduction
                ),
            })
            .collect(),
        noise: Some(NoiseState {
            action: 1,
            kind: 1,
            value: NoiseMode::Maximum as u8,
        }),
        noise_cycle: Some(NoiseState {
            action: 2,
            kind: 1,
            value: NoiseCycle::All as u8,
        }),
        eq_preset: Some(0),
        custom_eq: vec![CustomEq::air7(1, "Example EQ", [0, 1, 0, -1, 0, 1])],
        touch: [
            (1, 2, 1),
            (1, 3, 5),
            (1, 4, 8),
            (2, 2, 1),
            (2, 3, 6),
            (2, 4, 8),
        ]
        .into_iter()
        .map(|(ear, gesture, function)| TouchBinding {
            ear,
            button: 1,
            gesture,
            function,
        })
        .collect(),
        volume_enhancer: Some(false),
        dynamic_bands: (1..=3)
            .map(|kind| DynamicBand {
                min: -5,
                max: 5,
                value: 0,
                kind,
            })
            .collect(),
        versions: [
            (1, 2, "Example firmware"),
            (2, 2, "Example firmware"),
            (3, 2, "Example case firmware"),
        ]
        .into_iter()
        .map(|(component, kind, value)| Version {
            component,
            kind,
            value: value.into(),
        })
        .collect(),
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_has_no_host_identifiers() {
        let s = snapshot();
        assert!(s.multipoint.is_empty());
        assert_eq!(s.custom_eq[0].name, "Example EQ");
        assert_eq!(s.battery[0].percent, Some(84));
    }
    #[test]
    fn preview_uses_the_same_semantic_model() {
        let s = snapshot();
        assert_eq!(s.wearing_summary().scenario, WearingScenario::Both);
        assert_eq!(
            s.noise.unwrap().describe().category,
            Some(NoiseCategory::Anc)
        );
    }
}
