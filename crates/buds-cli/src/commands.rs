// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Command handlers delegate parsing, semantics and transport state to realme-buds.
use crate::args::*;
use realme_buds::{
    AncLevel, Client, CustomEq, EqAction, Feature, TouchBinding, Transport, describe_bands,
    describe_battery, describe_feature_ids, describe_features, describe_fit, describe_multipoint,
    describe_touch, describe_versions, describe_wear,
};
use std::time::Duration;

pub(crate) fn print_json(v: &impl serde::Serialize) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

fn print_view(
    raw: bool,
    wire: &impl serde::Serialize,
    named: &impl serde::Serialize,
) -> Result<(), serde_json::Error> {
    if raw {
        print_json(wire)
    } else {
        print_json(named)
    }
}
pub(crate) fn execute<T: Transport>(
    client: &mut Client<T>,
    command: Command,
    raw: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    client.initialize()?;
    match command {
        Command::Devices => unreachable!(),
        Command::Status => {
            let v = client.snapshot();
            print_view(raw, &v, &v.describe())?;
        }
        Command::Battery => {
            let v = client.battery()?;
            print_view(raw, &v, &describe_battery(&v))?;
        }
        Command::Versions => {
            let v = client.versions()?;
            print_view(raw, &v, &describe_versions(&v))?;
        }
        Command::Wear => {
            let v = client.wear()?;
            print_view(raw, &realme_buds::wearing_summary(&v), &describe_wear(&v))?;
        }
        Command::Firmware => {
            let v = client.firmware_info();
            print_view(raw, &v, &v.describe())?;
        }
        Command::Diagnostics => {
            let v = client.property_audit();
            let named: Vec<_> = v.iter().map(|v| v.describe()).collect();
            print_view(raw, &v, &named)?;
        }
        Command::FeatureIds { ids } => {
            let v = client.features_by_ids(&ids)?;
            print_view(raw, &v, &describe_feature_ids(&v, &ids))?;
        }
        Command::ToggleId { id, state } => {
            client.set_feature_id(id, state.enabled())?;
            let v = client.features_by_ids(&[id])?;
            print_view(raw, &v, &describe_feature_ids(&v, &[id]))?;
        }
        Command::Connection {
            address,
            action,
            confirm,
        } => {
            if !confirm {
                return Err(
                    "connection changes require --confirm; remove loses that host's pairing".into(),
                );
            }
            let bytes: Vec<_> = address
                .split(':')
                .map(|s| u8::from_str_radix(s, 16))
                .collect::<Result<_, _>>()?;
            let mac: [u8; 6] = bytes.try_into().map_err(|_| "invalid address")?;
            client.multipoint_action(
                mac,
                match action {
                    ConnectionArg::Disconnect => 0,
                    ConnectionArg::Connect => 1,
                    ConnectionArg::Remove => 2,
                },
            )?;
            let v = client.multipoint()?;
            print_view(raw, &v, &describe_multipoint(&v))?;
        }
        Command::Noise { mode, level } => {
            if let Some(m) = mode {
                client.set_noise(m.mode(level)?)?;
            } else if level.is_some() {
                return Err("specify noise anc with --level".into());
            }
            let v = client.noise()?;
            print_view(raw, &v, &v.describe())?;
        }
        Command::Anc { state, level } => {
            if let Some(s) = state {
                if !s.enabled() && level.is_some() {
                    return Err("anc off does not take --level".into());
                }
                client.set_anc_enabled(s.enabled(), level.map(AncLevel::from))?;
            } else if level.is_some() {
                return Err("specify anc on with --level".into());
            }
            let v = client.noise()?;
            print_view(raw, &v, &v.describe())?;
        }
        Command::Transparency { state } => {
            if let Some(s) = state {
                client.set_transparency_enabled(s.enabled())?;
            }
            let v = client.noise()?;
            print_view(raw, &v, &v.describe())?;
        }
        Command::Cycle { mode } => {
            if let Some(m) = mode {
                client.set_noise_cycle(m.into())?;
            }
            let v = client.noise_cycle()?;
            print_view(raw, &v, &realme_buds::cycle_identifier(v.value))?;
        }
        Command::Features => {
            let v = client.features()?;
            print_view(raw, &v, &describe_features(&v))?;
        }
        Command::Toggle { feature, state } => {
            let f: Feature = feature.into();
            client.set_feature(f, state.enabled())?;
            let enabled = client.feature(f)?;
            let named = describe_feature_ids(
                &[realme_buds::FeatureState {
                    id: f as u8,
                    enabled,
                }],
                &[f as u8],
            );
            print_view(raw, &enabled, &named)?;
        }
        Command::EqList => print_json(&client.custom_eq()?)?,
        Command::EqPreset { id } => {
            if let Some(id) = id {
                client.set_eq_preset(id)?;
            }
            let v = client.eq_preset()?;
            print_view(raw, &v, &realme_buds::preset_identifier(v))?;
        }
        Command::EqSave {
            id,
            name,
            gains,
            add,
        } => {
            let eq = CustomEq::air7(id, name, gains.try_into().map_err(|_| "need six gains")?);
            client.set_custom_eq(&eq, if add { EqAction::Add } else { EqAction::Update })?;
            print_json(&client.custom_eq()?)?;
        }
        Command::EqSelect { id } => {
            let eq = find_eq(client, id)?;
            client.set_custom_eq(&eq, EqAction::Update)?;
            print_json(&client.custom_eq()?)?;
        }
        Command::EqRename { id, name } => {
            let mut eq = find_eq(client, id)?;
            eq.name = name;
            client.set_custom_eq(&eq, EqAction::Update)?;
            print_json(&client.custom_eq()?)?;
        }
        Command::EqDelete { id, confirm } => {
            if !confirm {
                return Err("deleting EQ requires --confirm".into());
            }
            let eq = find_eq(client, id)?;
            client.set_custom_eq(&eq, EqAction::Delete)?;
            print_json(&client.custom_eq()?)?;
        }
        Command::Dynamic { band, value } => {
            if let Some(v) = value {
                let kind = band.ok_or("specify a band (1..3)")?;
                let mut b = client
                    .dynamic_bands()?
                    .into_iter()
                    .find(|b| b.kind == kind)
                    .ok_or("band not found")?;
                b.value = v;
                client.set_dynamic_band(b)?;
            }
            let v = client.dynamic_bands()?;
            print_view(raw, &v, &describe_bands(&v))?;
        }
        Command::Touch => {
            let v = client.touch()?;
            print_view(raw, &v, &describe_touch(&v))?;
        }
        Command::Bind {
            ear,
            gesture,
            function,
        } => {
            client.set_touch(&[TouchBinding {
                ear,
                button: 1,
                gesture,
                function,
            }])?;
            let v = client.touch()?;
            print_view(raw, &v, &describe_touch(&v))?;
        }
        Command::Volume {
            state,
            confirm_louder,
        } => {
            if let Some(s) = state {
                if s.enabled() && !confirm_louder {
                    return Err("enabling volume enhancer requires --confirm-louder".into());
                }
                client.set_volume_enhancer(s.enabled())?;
            }
            print_json(&client.volume_enhancer()?)?;
        }
        Command::Multipoint => {
            let v = client.multipoint()?;
            print_view(raw, &v, &describe_multipoint(&v))?;
        }
        Command::Watch => {
            client.subscribe()?;
            loop {
                match client.next_event(Duration::from_secs(30)) {
                    Ok(e) => print_view(raw, &e, &e.describe())?,
                    Err(realme_buds::Error::Timeout) => continue,
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Command::Find {
            state,
            confirm_sound,
        } => {
            if state.enabled() && !confirm_sound {
                return Err("requires --confirm-sound; remove earbuds from your ears first".into());
            }
            if state.enabled() {
                client.find_buds_for(Duration::from_secs(3))?;
                println!("start/stop acknowledged");
            } else {
                client.find_buds(false)?;
                println!("stop acknowledged");
            }
        }
        Command::Fit {
            state,
            confirm_sound,
        } => {
            if state.enabled() && !confirm_sound {
                return Err("requires --confirm-sound".into());
            }
            if state.enabled() {
                let v = client.run_fit_test(Duration::from_secs(15))?;
                print_view(raw, &v, &describe_fit(&v))?;
            } else {
                client.fit_test(false)?;
                println!("stop acknowledged");
            }
        }
    }
    Ok(())
}
fn find_eq<T: Transport>(
    c: &mut Client<T>,
    id: u8,
) -> Result<CustomEq, Box<dyn std::error::Error>> {
    c.custom_eq()?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "EQ id not found".into())
}
