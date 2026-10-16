// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Optional, driver-specific JSON command schema for C/other-language consumers.
//! This is not a universal Realme protocol. Typed Rust callers need not enable `json`.
use super::{advanced::ReadProperty, *};
use crate::Transport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub const SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Initialize {},
    Snapshot {},
    Battery {},
    Versions {},
    Wear {},
    Features {},
    FeatureIds {
        ids: Vec<u8>,
    },
    SetFeature {
        feature: Feature,
        enabled: bool,
    },
    SetFeatureId {
        id: u8,
        enabled: bool,
        confirm_experimental: bool,
    },
    Noise {},
    SetNoise {
        mode: NoiseMode,
    },
    SetAnc {
        enabled: bool,
        level: Option<AncLevel>,
    },
    SetTransparency {
        enabled: bool,
    },
    NoiseCycle {},
    SetNoiseCycle {
        cycle: NoiseCycle,
    },
    Touch {},
    SetTouch {
        bindings: Vec<TouchBinding>,
    },
    EqPreset {},
    SetEqPreset {
        id: u8,
    },
    CustomEq {},
    SetCustomEq {
        eq: CustomEq,
        action: EqAction,
    },
    DynamicBands {},
    SetDynamicBand {
        band: DynamicBand,
    },
    VolumeEnhancer {},
    SetVolumeEnhancer {
        enabled: bool,
        confirm_louder: bool,
    },
    Multipoint {},
    MultipointAction {
        address: [u8; 6],
        action: u8,
        confirm: bool,
    },
    Subscribe {},
    NextEvent {
        timeout_ms: u32,
    },
    RunFitTest {
        timeout_ms: u32,
        confirm_sound: bool,
    },
    StopFitTest {},
    FindFor {
        duration_ms: u32,
        confirm_sound: bool,
    },
    StopFinding {},
    Firmware {},
    Diagnostics {},
    ReadProperty {
        property: ReadProperty,
    },
    StatusFeatures {
        ids: Vec<u8>,
    },
    StoredData {
        ids: Vec<u8>,
    },
    UseTime {},
    SetAutoPowerOff {
        minutes: u8,
        confirm_experimental: bool,
    },
    /// Read-only diagnostic escape hatch. Arbitrary raw writes are deliberately not exposed.
    RawRead {
        command: u16,
        payload: Vec<u8>,
    },
}
#[derive(Debug, Serialize)]
pub struct Response {
    pub schema_version: u32,
    pub driver: &'static str,
    pub data: Value,
}
fn value(v: impl Serialize) -> Result<Value> {
    serde_json::to_value(v).map_err(|_| Error::Protocol("JSON serialization failed"))
}
fn confirm(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message))
    }
}
impl Request {
    pub fn is_initialize(&self) -> bool {
        matches!(self, Self::Initialize {})
    }
}
pub fn execute<T: Transport>(
    c: &mut Client<T>,
    request: Request,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Response> {
    let data = match request {
        Request::Initialize {} => value(c.initialize()?)?,
        Request::Snapshot {} => value(c.snapshot().describe())?,
        Request::Battery {} => value(describe_battery(&c.battery()?))?,
        Request::Versions {} => value(describe_versions(&c.versions()?))?,
        Request::Wear {} => value(describe_wear(&c.wear()?))?,
        Request::Features {} => value(describe_features(&c.features()?))?,
        Request::FeatureIds { ids } => {
            value(describe_feature_ids(&c.features_by_ids(&ids)?, &ids))?
        }
        Request::SetFeature { feature, enabled } => {
            c.set_feature(feature, enabled)?;
            Value::Null
        }
        Request::SetFeatureId {
            id,
            enabled,
            confirm_experimental,
        } => {
            confirm(
                confirm_experimental,
                "experimental SDK flag write needs confirmation",
            )?;
            c.set_feature_id(id, enabled)?;
            Value::Null
        }
        Request::Noise {} => value(c.noise()?.describe())?,
        Request::SetNoise { mode } => {
            c.set_noise(mode)?;
            Value::Null
        }
        Request::SetAnc { enabled, level } => {
            c.set_anc_enabled(enabled, level)?;
            Value::Null
        }
        Request::SetTransparency { enabled } => {
            c.set_transparency_enabled(enabled)?;
            Value::Null
        }
        Request::NoiseCycle {} => {
            let raw = c.noise_cycle()?;
            serde_json::json!({"cycle":cycle_identifier(raw.value),"raw":raw})
        }
        Request::SetNoiseCycle { cycle } => {
            c.set_noise_cycle(cycle)?;
            Value::Null
        }
        Request::Touch {} => value(describe_touch(&c.touch()?))?,
        Request::SetTouch { bindings } => {
            c.set_touch(&bindings)?;
            Value::Null
        }
        Request::EqPreset {} => value(preset_identifier(c.eq_preset()?))?,
        Request::SetEqPreset { id } => {
            c.set_eq_preset(id)?;
            Value::Null
        }
        Request::CustomEq {} => value(c.custom_eq()?)?,
        Request::SetCustomEq { eq, action } => {
            c.set_custom_eq(&eq, action)?;
            Value::Null
        }
        Request::DynamicBands {} => value(describe_bands(&c.dynamic_bands()?))?,
        Request::SetDynamicBand { band } => {
            c.set_dynamic_band(band)?;
            Value::Null
        }
        Request::VolumeEnhancer {} => value(c.volume_enhancer()?)?,
        Request::SetVolumeEnhancer {
            enabled,
            confirm_louder,
        } => {
            confirm(
                !enabled || confirm_louder,
                "volume increase needs explicit confirmation",
            )?;
            c.set_volume_enhancer(enabled)?;
            Value::Null
        }
        Request::Multipoint {} => value(describe_multipoint(&c.multipoint()?))?,
        Request::MultipointAction {
            address,
            action,
            confirm: yes,
        } => {
            confirm(yes, "multipoint changes need confirmation")?;
            c.multipoint_action(address, action)?;
            Value::Null
        }
        Request::Subscribe {} => {
            c.subscribe()?;
            Value::Null
        }
        Request::NextEvent { timeout_ms } => {
            if timeout_ms == 0 {
                return Err(Error::Invalid("event timeout must be nonzero"));
            }
            value(
                c.next_event(Duration::from_millis(timeout_ms.into()))?
                    .describe(),
            )?
        }
        Request::RunFitTest {
            timeout_ms,
            confirm_sound,
        } => {
            confirm(confirm_sound, "fit sound needs explicit confirmation")?;
            value(describe_fit(&c.run_fit_test_with_cancel(
                Duration::from_millis(timeout_ms.into()),
                &mut cancelled,
            )?))?
        }
        Request::StopFitTest {} => {
            c.fit_test(false)?;
            Value::Null
        }
        Request::FindFor {
            duration_ms,
            confirm_sound,
        } => {
            confirm(
                confirm_sound,
                "remove both earbuds and explicitly confirm sound",
            )?;
            c.find_buds_for_with_cancel(Duration::from_millis(duration_ms.into()), &mut cancelled)?;
            Value::Null
        }
        Request::StopFinding {} => {
            c.find_buds(false)?;
            Value::Null
        }
        Request::Firmware {} => value(c.firmware_info().describe())?,
        Request::Diagnostics {} => value(
            c.property_audit()
                .iter()
                .map(|p| p.describe())
                .collect::<Vec<_>>(),
        )?,
        Request::ReadProperty { property } => value(c.read_property(property)?)?,
        Request::StatusFeatures { ids } => value(c.status_features(&ids)?)?,
        Request::StoredData { ids } => value(c.stored_data(&ids)?)?,
        Request::UseTime {} => value(c.use_time()?)?,
        Request::SetAutoPowerOff {
            minutes,
            confirm_experimental,
        } => {
            confirm(
                confirm_experimental,
                "experimental auto-off write needs confirmation",
            )?;
            c.set_auto_power_off(minutes)?;
            Value::Null
        }
        Request::RawRead { command, payload } => {
            if !matches!(
                command,
                0x0105
                    | 0x0106
                    | 0x0107
                    | 0x0108
                    | 0x0109
                    | 0x010b
                    | 0x010c
                    | 0x010d
                    | 0x010f
                    | 0x0110
                    | 0x0112
                    | 0x0122
                    | 0x012c
                    | 0x0200
            ) {
                return Err(Error::Invalid(
                    "raw read not allowlisted; use a named SDK property read",
                ));
            }
            value(c.request(command, payload)?)?
        }
    };
    Ok(Response {
        schema_version: SCHEMA_VERSION,
        driver: DRIVER_ID,
        data,
    })
}
