// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Shared semantic descriptions. These derive names/states from observed transport data;
//! they never invent measurements, feature support, or a stored ANC level while ANC is off.
use super::{Client, Error, Result, model::*};
use crate::Transport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identifier {
    pub id: u8,
    pub name: String,
    /// A named SDK/protocol identifier; not a claim of verified Air7 behavior.
    pub identified: bool,
}
fn identify(id: u8, name: Option<&str>, kind: &str) -> Identifier {
    Identifier {
        id,
        name: name
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Unidentified {kind} (ID {id})")),
        identified: name.is_some(),
    }
}
impl Component {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::Left),
            2 => Some(Self::Right),
            3 => Some(Self::Case),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "Left earbud",
            Self::Right => "Right earbud",
            Self::Case => "Charging case",
        }
    }
    pub fn describe(id: u8) -> Identifier {
        identify(id, Self::from_id(id).map(Self::name), "component")
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AncLevel {
    Mild,
    Moderate,
    Maximum,
    Adaptive,
}
impl AncLevel {
    pub const ALL: [Self; 4] = [Self::Mild, Self::Moderate, Self::Maximum, Self::Adaptive];
    pub fn from_wire(value: u8) -> Option<Self> {
        match value {
            4 => Some(Self::Mild),
            16 => Some(Self::Moderate),
            8 => Some(Self::Maximum),
            32 => Some(Self::Adaptive),
            _ => None,
        }
    }
    pub fn mode(self) -> NoiseMode {
        match self {
            Self::Mild => NoiseMode::Mild,
            Self::Moderate => NoiseMode::Moderate,
            Self::Maximum => NoiseMode::Maximum,
            Self::Adaptive => NoiseMode::Adaptive,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Mild => "Mild",
            Self::Moderate => "Moderate",
            Self::Maximum => "Maximum",
            Self::Adaptive => "Adaptive",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NoiseCategory {
    Off,
    Transparency,
    Anc,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoiseDescription {
    pub category: Option<NoiseCategory>,
    pub anc_enabled: Option<bool>,
    pub transparency_enabled: Option<bool>,
    /// Only the active ANC setting. There is no inferred last-used setting when off/transparency.
    pub active_anc_level: Option<AncLevel>,
    pub raw: NoiseState,
}
impl NoiseState {
    pub fn describe(self) -> NoiseDescription {
        let level = if self.action == 1 && self.kind == 1 {
            AncLevel::from_wire(self.value)
        } else {
            None
        };
        let category = if self.action != 1 || self.kind != 1 {
            None
        } else {
            match self.value {
                1 => Some(NoiseCategory::Off),
                2 => Some(NoiseCategory::Transparency),
                _ => level.map(|_| NoiseCategory::Anc),
            }
        };
        NoiseDescription {
            category,
            anc_enabled: category.map(|v| v == NoiseCategory::Anc),
            transparency_enabled: category.map(|v| v == NoiseCategory::Transparency),
            active_anc_level: level,
            raw: self,
        }
    }
}
/// Host/UI memory, NOT a read-back earbud property. Scope this per device, not globally.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AncPreference {
    pub preferred_level: Option<AncLevel>,
}
impl AncPreference {
    pub fn observe(&mut self, state: NoiseState) {
        if let Some(level) = state.describe().active_anc_level {
            self.preferred_level = Some(level);
        }
    }
    /// Local choice alone performs no Bluetooth operation, and is not a confirmed device state.
    pub fn choose(&mut self, level: AncLevel) {
        self.preferred_level = Some(level);
    }
}
impl<T: Transport> Client<T> {
    pub fn noise_description(&mut self) -> Result<NoiseDescription> {
        Ok(self.noise()?.describe())
    }
    /// The UI's ANC toggle selects the single wire mode. Enabling requires an explicit level;
    /// disabling selects Off (it does not preserve Transparency). No guessed default is used.
    pub fn set_anc_enabled(&mut self, enabled: bool, level: Option<AncLevel>) -> Result<()> {
        let mode = if enabled {
            level
                .ok_or(Error::Invalid(
                    "ANC on requires an explicit level; last-used level is a host/UI preference",
                ))?
                .mode()
        } else {
            NoiseMode::Off
        };
        self.set_noise(mode)
    }
    /// Transparency is another mutually exclusive wire mode, not an independent boolean.
    pub fn set_transparency_enabled(&mut self, enabled: bool) -> Result<()> {
        self.set_noise(if enabled {
            NoiseMode::Transparency
        } else {
            NoiseMode::Off
        })
    }
}
impl Feature {
    pub fn from_id(id: u8) -> Option<Self> {
        Self::AIR7
            .iter()
            .find(|(f, _)| *f as u8 == id)
            .map(|(f, _)| *f)
    }
    pub fn name(self) -> &'static str {
        Self::AIR7
            .iter()
            .find(|(f, _)| *f == self)
            .expect("named feature")
            .1
    }
}
pub fn feature_identifier(id: u8) -> Identifier {
    let name = Feature::from_id(id).map(Feature::name).or(match id {
        5 => Some("Power-saving mode (generic SDK; Air7 effect unverified)"),
        7 => Some("Party mode (generic SDK; Air7 effect unverified)"),
        9 => Some("Voice enhancement, SDK flag 9 (Air7 effect unverified)"),
        11 => Some("HearID enable (generic SDK; Air7 effect unverified)"),
        12 => Some("Personalized ANC enable (generic SDK; Air7 effect unverified)"),
        54 => Some("Virtual bass (generic SDK; Air7 effect unverified)"),
        _ => None,
    });
    identify(id, name, "SDK feature")
}
pub fn version_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            1 => Some("Hardware version"),
            2 => Some("Firmware version"),
            3 => Some("Auxiliary SDK version / upgrade marker (meaning partly unresolved)"),
            4 => Some("Vendor version"),
            _ => None,
        },
        "version kind",
    )
}
pub fn gesture_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            1 => Some("Single tap"),
            2 => Some("Double tap"),
            3 => Some("Triple tap"),
            4 => Some("Hold"),
            6 => Some("Long hold"),
            _ => None,
        },
        "gesture",
    )
}
pub fn touch_function_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            0 => Some("Disabled"),
            1 => Some("Play / pause"),
            3 => Some("Voice assistant (legacy SDK encoding; host-dependent)"),
            4 => Some("Voice assistant (host-dependent)"),
            5 => Some("Previous track"),
            6 => Some("Next track"),
            8 => Some("Cycle noise control"),
            11 => Some("Volume up"),
            12 => Some("Volume down"),
            13 => Some("Switch connected device (generic SDK; Air7 effect unverified)"),
            17 => Some("Game mode"),
            26 => Some("Host action 26 (SDK conversation translation; Air7 meaning unverified)"),
            27 => Some("Host action 27 (SDK live translation; Air7 meaning unverified)"),
            31 => Some("AI assistant (requires compatible host; Air7 effect unverified)"),
            _ => None,
        },
        "touch function",
    )
}
pub fn band_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            1 => Some("Low"),
            2 => Some("Mid"),
            3 => Some("High"),
            _ => None,
        },
        "dynamic audio band",
    )
}
pub fn preset_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            0 => Some("Original sound"),
            1 => Some("Serenade"),
            2 => Some("Clear Bass"),
            3 => Some("Deep Bass"),
            _ => None,
        },
        "EQ preset",
    )
}
pub fn multipoint_state_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            0 => Some("Disconnected"),
            1 => Some("Connecting"),
            2 => Some("Connected"),
            _ => None,
        },
        "multipoint state",
    )
}
pub fn cycle_identifier(id: u8) -> Identifier {
    identify(
        id,
        match id {
            3 => Some("Off / Transparency"),
            9 => Some("Off / ANC"),
            10 => Some("ANC / Transparency"),
            11 => Some("Off / ANC / Transparency"),
            _ => None,
        },
        "noise cycle",
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Availability {
    Reported,
    NotReported,
    ReadFailed,
}
#[derive(Debug, Clone, Serialize)]
pub struct BatteryDescription {
    pub component: Identifier,
    pub availability: Availability,
    pub percent: Option<u8>,
    /// None if there was no observation. A raw charging bit is not a freshness guarantee.
    pub charging_flag: Option<bool>,
    pub raw: Option<Battery>,
}
pub fn describe_battery(readings: &[Battery]) -> Vec<BatteryDescription> {
    let mut ids = vec![1, 2, 3];
    for b in readings {
        if !ids.contains(&b.component) {
            ids.push(b.component);
        }
    }
    ids.into_iter()
        .map(|id| {
            let raw = readings.iter().find(|b| b.component == id).copied();
            BatteryDescription {
                component: Component::describe(id),
                availability: if raw.is_some() {
                    Availability::Reported
                } else {
                    Availability::NotReported
                },
                percent: raw.and_then(|b| b.percent),
                charging_flag: raw.map(|b| b.charging),
                raw,
            }
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct WearDescription {
    pub component: Identifier,
    pub availability: Availability,
    /// Only ear components have identified physical wearing/in-case interpretations.
    pub worn: Option<bool>,
    pub in_case: Option<bool>,
    pub connection_flag: Option<bool>,
    pub raw: Option<WearState>,
}
pub fn describe_wear(readings: &[WearState]) -> Vec<WearDescription> {
    let mut ids = vec![1, 2, 3];
    for w in readings {
        if !ids.contains(&w.component) {
            ids.push(w.component);
        }
    }
    ids.into_iter()
        .map(|id| {
            let raw = readings.iter().find(|w| w.component == id).copied();
            WearDescription {
                component: Component::describe(id),
                availability: if raw.is_some() {
                    Availability::Reported
                } else {
                    Availability::NotReported
                },
                worn: raw.filter(|_| matches!(id, 1 | 2)).map(|w| w.worn),
                in_case: raw.filter(|_| matches!(id, 1 | 2)).map(|w| w.in_case),
                connection_flag: raw.map(|w| w.connected),
                raw,
            }
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct VersionDescription {
    pub component: Identifier,
    pub kind: Identifier,
    pub value: String,
    pub raw: Version,
}
pub fn describe_versions(v: &[Version]) -> Vec<VersionDescription> {
    v.iter()
        .map(|v| VersionDescription {
            component: Component::describe(v.component),
            kind: version_identifier(v.kind),
            value: v.value.clone(),
            raw: v.clone(),
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct FeatureDescription {
    pub feature: Identifier,
    pub availability: Availability,
    pub enabled: Option<bool>,
    pub raw: Option<FeatureState>,
}
pub fn describe_features(v: &[FeatureState]) -> Vec<FeatureDescription> {
    describe_feature_ids(v, super::commands::FEATURE_IDS)
}
pub fn describe_feature_ids(v: &[FeatureState], requested: &[u8]) -> Vec<FeatureDescription> {
    let mut ids = requested.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for f in v {
        if !ids.contains(&f.id) {
            ids.push(f.id);
        }
    }
    ids.into_iter()
        .map(|id| {
            let raw = v.iter().find(|v| v.id == id).copied();
            FeatureDescription {
                feature: feature_identifier(id),
                availability: if raw.is_some() {
                    Availability::Reported
                } else {
                    Availability::NotReported
                },
                enabled: raw.map(|f| f.enabled),
                raw,
            }
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct TouchDescription {
    pub component: Identifier,
    pub gesture: Identifier,
    pub function: Identifier,
    pub raw: TouchBinding,
}
pub fn describe_touch(v: &[TouchBinding]) -> Vec<TouchDescription> {
    v.iter()
        .map(|&v| TouchDescription {
            component: Component::describe(v.ear),
            gesture: gesture_identifier(v.gesture),
            function: touch_function_identifier(v.function),
            raw: v,
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct BandDescription {
    pub band: Identifier,
    pub minimum: i8,
    pub maximum: i8,
    pub value: i8,
    pub raw: DynamicBand,
}
pub fn describe_bands(v: &[DynamicBand]) -> Vec<BandDescription> {
    v.iter()
        .map(|&v| BandDescription {
            band: band_identifier(v.kind),
            minimum: v.min,
            maximum: v.max,
            value: v.value,
            raw: v,
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct FitDescription {
    pub component: Identifier,
    pub outcome: FitOutcome,
    pub raw: FitResult,
}
pub fn describe_fit(v: &[FitResult]) -> Vec<FitDescription> {
    v.iter()
        .map(|v| FitDescription {
            component: Component::describe(v.ear),
            outcome: v.outcome(),
            raw: v.clone(),
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct MultipointDescription {
    pub name: String,
    pub address: String,
    pub state: Identifier,
    pub sdk_my_phone_flag: bool,
    pub raw: MultipointDevice,
}
pub fn describe_multipoint(v: &[MultipointDevice]) -> Vec<MultipointDescription> {
    v.iter()
        .map(|v| MultipointDescription {
            name: v.name.clone(),
            address: v.address.clone(),
            state: multipoint_state_identifier(v.state),
            sdk_my_phone_flag: v.current,
            raw: v.clone(),
        })
        .collect()
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum SoundSelection {
    Stock { preset: Identifier },
    Custom { id: u8, name: String },
    ConflictingCustomSelections(Vec<u8>),
    Unknown,
}
impl Snapshot {
    pub fn sound_selection(&self) -> SoundSelection {
        let selected: Vec<_> = self.custom_eq.iter().filter(|e| e.selected).collect();
        match selected.as_slice() {
            [] => self
                .eq_preset
                .map(|id| SoundSelection::Stock {
                    preset: preset_identifier(id),
                })
                .unwrap_or(SoundSelection::Unknown),
            [eq] => SoundSelection::Custom {
                id: eq.id,
                name: eq.name.clone(),
            },
            _ => {
                SoundSelection::ConflictingCustomSelections(selected.iter().map(|e| e.id).collect())
            }
        }
    }
    pub fn describe(&self) -> SnapshotDescription {
        let mut battery = describe_battery(&self.battery);
        let mut wear = describe_wear(&self.wear);
        let mut features = describe_features(&self.features);
        if self.errors.iter().any(|e| e.starts_with("battery:")) {
            for v in &mut battery {
                if v.raw.is_none() {
                    v.availability = Availability::ReadFailed;
                }
            }
        }
        if self.errors.iter().any(|e| e.starts_with("wear:")) {
            for v in &mut wear {
                if v.raw.is_none() {
                    v.availability = Availability::ReadFailed;
                }
            }
        }
        if self.errors.iter().any(|e| e.starts_with("features:")) {
            for v in &mut features {
                if v.raw.is_none() {
                    v.availability = Availability::ReadFailed;
                }
            }
        }
        SnapshotDescription {
            battery,
            wear,
            wearing_scenario: self.wearing_summary().scenario,
            noise: self.noise.map(NoiseState::describe),
            noise_cycle: self.noise_cycle.map(|v| cycle_identifier(v.value)),
            features,
            versions: describe_versions(&self.versions),
            sound_selection: self.sound_selection(),
            custom_eq: self.custom_eq.clone(),
            touch: describe_touch(&self.touch),
            volume_enhancer: self.volume_enhancer,
            dynamic_bands: describe_bands(&self.dynamic_bands),
            multipoint: describe_multipoint(&self.multipoint),
            raw: self.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotDescription {
    pub battery: Vec<BatteryDescription>,
    pub wear: Vec<WearDescription>,
    pub wearing_scenario: WearingScenario,
    pub noise: Option<NoiseDescription>,
    pub noise_cycle: Option<Identifier>,
    pub features: Vec<FeatureDescription>,
    pub versions: Vec<VersionDescription>,
    pub sound_selection: SoundSelection,
    pub custom_eq: Vec<CustomEq>,
    pub touch: Vec<TouchDescription>,
    pub volume_enhancer: Option<bool>,
    pub dynamic_bands: Vec<BandDescription>,
    pub multipoint: Vec<MultipointDescription>,
    pub raw: Snapshot,
}
