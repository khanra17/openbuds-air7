// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Named diagnostic/event descriptions; unresolved payload meanings stay explicitly opaque.
use crate::{advanced::*, semantics::*, *};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UpgradeDescription {
    pub component: Identifier,
    pub supported: bool,
    pub upgrade_type_raw: u8,
    pub raw: UpgradeCapability,
}
fn upgrades(v: &[UpgradeCapability]) -> Vec<UpgradeDescription> {
    v.iter()
        .map(|v| UpgradeDescription {
            component: Component::describe(v.component),
            supported: v.supported,
            upgrade_type_raw: v.upgrade_type,
            raw: v.clone(),
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct FirmwareDescription {
    pub versions: Vec<VersionDescription>,
    pub upgrade_capabilities: Vec<UpgradeDescription>,
    pub vendor_id_hex: Option<String>,
    pub product_id_hex: Option<String>,
    pub protocol_bytes: Vec<u8>,
    pub raw: FirmwareInfo,
}
impl FirmwareInfo {
    pub fn describe(&self) -> FirmwareDescription {
        FirmwareDescription {
            versions: describe_versions(&self.versions),
            upgrade_capabilities: upgrades(&self.upgrade_capabilities),
            vendor_id_hex: self.vendor_id.map(|v| format!("0x{v:04X}")),
            product_id_hex: self.product_id.map(|v| format!("0x{v:06X}")),
            protocol_bytes: self.protocol_version.clone(),
            raw: self.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub enum DiagnosticValue {
    VendorId(String),
    ProductId(String),
    ProtocolBytes(Vec<u8>),
    Versions(Vec<VersionDescription>),
    UpgradeCapabilities(Vec<UpgradeDescription>),
    ColorCodeUnidentified(u8),
    LanguageCodeUnidentified(u8),
    PersonalizedAncProfileExists(bool),
    CustomEq(Vec<CustomEq>),
    DynamicBands(Vec<BandDescription>),
    AdaptiveLevel {
        reported_level: Option<AncLevel>,
        raw: NoiseState,
    },
    StatusFeaturesUnidentified(Vec<StatusFeature>),
    OpaqueBytes(Vec<u8>),
}
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticDescription {
    pub property: ReadProperty,
    pub advertised: bool,
    pub value: Option<DiagnosticValue>,
    pub interpretation_error: Option<String>,
    pub raw: PropertyResult,
}
impl PropertyResult {
    pub fn describe(&self) -> DiagnosticDescription {
        let decoded = self.data.as_ref().map(|b| -> Result<DiagnosticValue> {
            Ok(match self.property {
                ReadProperty::VendorId if b.len() == 2 => {
                    DiagnosticValue::VendorId(format!("0x{:04X}", u16::from_le_bytes([b[0], b[1]])))
                }
                ReadProperty::ProductId if b.len() == 3 => DiagnosticValue::ProductId(format!(
                    "0x{:06X}",
                    u32::from_le_bytes([b[0], b[1], b[2], 0])
                )),
                ReadProperty::ProtocolVersion => DiagnosticValue::ProtocolBytes(b.clone()),
                ReadProperty::FirmwareVersions => {
                    DiagnosticValue::Versions(describe_versions(&model::parse_versions(b)?))
                }
                ReadProperty::UpgradeCapabilities => {
                    DiagnosticValue::UpgradeCapabilities(upgrades(&parse_upgrade_capabilities(b)?))
                }
                ReadProperty::Color if b.len() == 1 => DiagnosticValue::ColorCodeUnidentified(b[0]),
                ReadProperty::Language if b.len() == 1 => {
                    DiagnosticValue::LanguageCodeUnidentified(b[0])
                }
                ReadProperty::PersonalizedAncExists if b.len() == 1 && b[0] <= 1 => {
                    DiagnosticValue::PersonalizedAncProfileExists(b[0] != 0)
                }
                ReadProperty::CustomEq => DiagnosticValue::CustomEq(model::parse_custom_eq(b)?),
                ReadProperty::DynamicBands => {
                    DiagnosticValue::DynamicBands(describe_bands(&model::parse_dynamic_bands(b)?))
                }
                ReadProperty::NoiseAdaptiveLevel => {
                    let raw = model::parse_noise(b)?;
                    DiagnosticValue::AdaptiveLevel {
                        reported_level: if raw.action == 4 && raw.kind == 1 {
                            AncLevel::from_wire(raw.value)
                        } else {
                            None
                        },
                        raw,
                    }
                }
                ReadProperty::FeatureStatus => {
                    DiagnosticValue::StatusFeaturesUnidentified(parse_status_features(b)?)
                }
                _ => DiagnosticValue::OpaqueBytes(b.clone()),
            })
        });
        DiagnosticDescription {
            property: self.property,
            advertised: self.advertised,
            value: decoded.as_ref().and_then(|r| r.as_ref().ok()).cloned(),
            interpretation_error: decoded.and_then(|r| r.err()).map(|e| e.to_string()),
            raw: self.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub enum EventDescription {
    Battery(Vec<BatteryDescription>),
    Wear(Vec<WearDescription>),
    Noise(NoiseDescription),
    NoiseCycle {
        cycle: Identifier,
        raw: NoiseState,
    },
    OtherNoiseReport {
        raw: NoiseState,
    },
    Fit(Vec<FitDescription>),
    Features(Vec<FeatureDescription>),
    EqPreset(Identifier),
    CustomEq(Vec<CustomEq>),
    DynamicBand(BandDescription),
    Multipoint(Vec<MultipointDescription>),
    HostOperation {
        operation: Identifier,
    },
    UserInteraction {
        component: Identifier,
        gesture: Identifier,
        function: Identifier,
        raw: UserInteraction,
    },
    UnidentifiedDiagnosticStatus(Vec<StatusFeature>),
    UnidentifiedFeatureDetection(StatusFeature),
    UseTime {
        raw: Vec<UseTime>,
        unit_unidentified: bool,
    },
    Raw(codec::Packet),
}
impl Event {
    pub fn describe(&self) -> EventDescription {
        match self {
            Self::Battery(v) => EventDescription::Battery(describe_battery(v)),
            Self::Wear(v) => EventDescription::Wear(describe_wear(v)),
            Self::Noise(v) if v.action == 1 => EventDescription::Noise(v.describe()),
            Self::Noise(v) if v.action == 2 => EventDescription::NoiseCycle {
                cycle: cycle_identifier(v.value),
                raw: *v,
            },
            Self::Noise(v) => EventDescription::OtherNoiseReport { raw: *v },
            Self::Fit(v) => EventDescription::Fit(describe_fit(v)),
            Self::Features(v) => EventDescription::Features(describe_feature_ids(
                v,
                &v.iter().map(|v| v.id).collect::<Vec<_>>(),
            )),
            Self::EqPreset(v) => EventDescription::EqPreset(preset_identifier(*v)),
            Self::CustomEq(v) => EventDescription::CustomEq(v.clone()),
            Self::DynamicBand(v) => EventDescription::DynamicBand(describe_bands(&[*v]).remove(0)),
            Self::Multipoint(v) => EventDescription::Multipoint(describe_multipoint(v)),
            Self::HostOperation(v) => EventDescription::HostOperation {
                operation: Identifier {
                    id: *v,
                    name: match v {
                        5 => "Find phone start request (host implementation required)",
                        6 => "Find phone stop request (host implementation required)",
                        _ => "Unidentified host operation",
                    }
                    .into(),
                    identified: matches!(v, 5 | 6),
                },
            },
            Self::UserInteraction(v) => EventDescription::UserInteraction {
                component: Component::describe(v.component),
                gesture: gesture_identifier(v.gesture),
                function: touch_function_identifier(v.function),
                raw: v.clone(),
            },
            Self::StatusFeatures(v) => EventDescription::UnidentifiedDiagnosticStatus(v.clone()),
            Self::FeatureDetection(v) => EventDescription::UnidentifiedFeatureDetection(v.clone()),
            Self::UseTime(v) => EventDescription::UseTime {
                raw: v.clone(),
                unit_unidentified: true,
            },
            Self::Raw(v) => EventDescription::Raw(v.clone()),
        }
    }
}
