// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use super::{Error, Result, codec::Packet, commands as cmd};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub bitmap: Vec<u8>,
    pub commands: BTreeSet<u16>,
}
impl Capabilities {
    pub fn parse(data: &[u8]) -> Self {
        let commands = cmd::CAPABILITY_COMMANDS
            .iter()
            .enumerate()
            .filter(|(i, _)| data.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0))
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        Self {
            bitmap: data.to_vec(),
            commands,
        }
    }
    pub fn supports(&self, command: u16) -> bool {
        command <= 0x0104
            || command == cmd::NOTIFICATION_CAPABILITIES
            || self.commands.contains(&command)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum Feature {
    AutoPlayPause = 4,
    GameMode = 6,
    AutoAnswer = 8,
    DualDevice = 17,
    HighDefinitionSound = 24,
    WindNoiseReduction = 26,
    SpatialAudio = 27,
    DynamicAudio = 29,
    /// Enables the fixed four-tap find-phone gesture; ringing is implemented by the host.
    FindPhoneGesture = 38,
}
impl Feature {
    pub const AIR7: &'static [(Feature, &'static str)] = &[
        (Self::AutoPlayPause, "Auto play/pause"),
        (Self::AutoAnswer, "Auto-answer"),
        (Self::GameMode, "Game mode"),
        (Self::DualDevice, "Dual-device connection"),
        (Self::HighDefinitionSound, "High-definition sound (LHDC)"),
        (Self::WindNoiseReduction, "Wind noise reduction"),
        (Self::SpatialAudio, "Spatial Audio"),
        (Self::DynamicAudio, "Dynamic audio"),
        (
            Self::FindPhoneGesture,
            "Find-phone four-tap gesture (host support required)",
        ),
    ];
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum NoiseMode {
    Off = 1,
    Transparency = 2,
    Mild = 4,
    Maximum = 8,
    Moderate = 16,
    Adaptive = 32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum NoiseCycle {
    OffTransparency = 3,
    OffCancellation = 9,
    CancellationTransparency = 10,
    All = 11,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoiseState {
    pub action: u8,
    pub kind: u8,
    pub value: u8,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureState {
    pub id: u8,
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Battery {
    pub component: u8,
    /// None when the firmware reports an unavailable/non-percentage value.
    pub percent: Option<u8>,
    pub charging: bool,
    pub raw: u8,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WearState {
    pub component: u8,
    pub in_case: bool,
    pub worn: bool,
    pub connected: bool,
    pub raw: u8,
}
/// Component IDs verified by the user's right-only wearing transition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum Component {
    Left = 1,
    Right = 2,
    Case = 3,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WearingScenario {
    Unknown,
    Neither,
    LeftOnly,
    RightOnly,
    Both,
}
/// Missing/offline ears remain explicit rather than being assumed not worn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WearingSummary {
    pub left: Option<WearState>,
    pub right: Option<WearState>,
    pub case: Option<WearState>,
    pub scenario: WearingScenario,
}
pub fn wearing_summary(states: &[WearState]) -> WearingSummary {
    let get = |id| states.iter().find(|s| s.component == id).copied();
    let left = get(1);
    let right = get(2);
    let case = get(3);
    let scenario = match (left, right) {
        (Some(l), Some(r)) if l.connected && r.connected => match (l.worn, r.worn) {
            (false, false) => WearingScenario::Neither,
            (true, false) => WearingScenario::LeftOnly,
            (false, true) => WearingScenario::RightOnly,
            (true, true) => WearingScenario::Both,
        },
        _ => WearingScenario::Unknown,
    };
    WearingSummary {
        left,
        right,
        case,
        scenario,
    }
}
impl Snapshot {
    pub fn wearing_summary(&self) -> WearingSummary {
        wearing_summary(&self.wear)
    }
    /// Missing readings remain None, even if a cached wear/connection flag exists.
    pub fn battery_for(&self, component: Component) -> Option<&Battery> {
        self.battery.iter().find(|b| b.component == component as u8)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub component: u8,
    pub kind: u8,
    pub value: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TouchBinding {
    pub ear: u8,
    pub button: u8,
    pub gesture: u8,
    pub function: u8,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum Gesture {
    SingleTap = 1,
    DoubleTap = 2,
    TripleTap = 3,
    Hold = 4,
    LongHold = 6,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum TouchFunction {
    Disabled = 0,
    PlayPause = 1,
    VoiceAssistant = 4,
    PreviousTrack = 5,
    NextTrack = 6,
    NoiseControl = 8,
    VolumeUp = 11,
    VolumeDown = 12,
    SwitchDevice = 13,
    GameMode = 17,
    /// Context-dependent SDK host action; do not assume this means find-phone on Air7.
    HostAction26 = 26,
    HostAction27 = 27,
    AiAssistant = 31,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EqBand {
    pub frequency: u16,
    pub gain: i8,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomEq {
    pub selected: bool,
    pub min: i8,
    pub max: i8,
    pub id: u8,
    pub name: String,
    pub bands: Vec<EqBand>,
}
impl CustomEq {
    pub fn air7(id: u8, name: impl Into<String>, gains: [i8; 6]) -> Self {
        Self {
            selected: false,
            min: -6,
            max: 6,
            id,
            name: name.into(),
            bands: [62, 250, 1000, 4000, 8000, 16000]
                .into_iter()
                .zip(gains)
                .map(|(frequency, gain)| EqBand { frequency, gain })
                .collect(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.name.len() > 255 || self.name.contains('\0') {
            return Err(Error::Invalid(
                "EQ name must be <=255 UTF-8 bytes, without NUL",
            ));
        }
        if self.min > self.max || self.bands.is_empty() || self.bands.len() > 255 {
            return Err(Error::Invalid("invalid EQ bounds/bands"));
        }
        let mut frequencies = BTreeSet::new();
        for b in &self.bands {
            if b.frequency == 0
                || !frequencies.insert(b.frequency)
                || !(self.min..=self.max).contains(&b.gain)
            {
                return Err(Error::Invalid("EQ frequency/gain outside bounds"));
            }
        }
        Ok(())
    }
    pub fn set_payload(&self, action: EqAction) -> Result<Vec<u8>> {
        self.validate()?;
        let mut b = vec![
            action as u8,
            self.min as u8,
            self.max as u8,
            self.id,
            self.name.len() as u8,
        ];
        b.extend(self.name.as_bytes());
        b.push(self.bands.len() as u8);
        for band in &self.bands {
            b.extend(band.frequency.to_le_bytes());
            b.push(band.gain as u8);
        }
        Ok(b)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum EqAction {
    Add = 1,
    Update = 2,
    Delete = 3,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicBand {
    pub min: i8,
    pub max: i8,
    pub value: i8,
    pub kind: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultipointDevice {
    pub address: String,
    /// 0 disconnected, 1 connecting, 2 connected; other values remain raw.
    pub state: u8,
    /// SDK calls flag 1 'my phone'; it is not proof this host owns our control socket.
    pub current: bool,
    pub flags: u8,
    pub name: String,
    pub name_bytes: Vec<u8>,
    pub extension: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FitResult {
    pub ear: u8,
    /// Raw SDK status; 0/1 are final fit verdicts, not progress markers.
    pub result: u8,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FitOutcome {
    GeneralFit,
    GoodFit,
    DetectionFailed,
    InterruptedByWear,
    InterruptedByApp,
    Unknown(u8),
}
impl FitResult {
    pub fn outcome(&self) -> FitOutcome {
        match self.result {
            0 => FitOutcome::GeneralFit,
            1 => FitOutcome::GoodFit,
            3 => FitOutcome::DetectionFailed,
            4 => FitOutcome::InterruptedByWear,
            5 => FitOutcome::InterruptedByApp,
            v => FitOutcome::Unknown(v),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserInteraction {
    pub component: u8,
    pub button: u8,
    pub gesture: u8,
    pub function: u8,
    pub scene: u8,
    pub options: Vec<u16>,
}
pub fn parse_user_interaction(data: &[u8]) -> Result<UserInteraction> {
    let mut c = Cursor::new(data);
    let mut v = UserInteraction {
        component: c.u8()?,
        button: c.u8()?,
        gesture: c.u8()?,
        function: c.u8()?,
        scene: c.u8()?,
        options: Vec::new(),
    };
    while c.offset < c.data.len() {
        v.options.push(c.u16()?);
    }
    Ok(v)
}

/// Raw unknown notifications are retained so new firmware is not silently misinterpreted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    StatusFeatures(Vec<super::advanced::StatusFeature>),
    FeatureDetection(super::advanced::StatusFeature),
    UseTime(Vec<super::advanced::UseTime>),
    UserInteraction(UserInteraction),
    HostOperation(u8),
    Battery(Vec<Battery>),
    Wear(Vec<WearState>),
    Noise(NoiseState),
    Fit(Vec<FitResult>),
    Features(Vec<FeatureState>),
    EqPreset(u8),
    CustomEq(Vec<CustomEq>),
    DynamicBand(DynamicBand),
    Multipoint(Vec<MultipointDevice>),
    Raw(Packet),
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub capabilities: Capabilities,
    pub battery: Vec<Battery>,
    pub versions: Vec<Version>,
    pub wear: Vec<WearState>,
    pub features: Vec<FeatureState>,
    pub noise: Option<NoiseState>,
    pub noise_cycle: Option<NoiseState>,
    pub touch: Vec<TouchBinding>,
    pub eq_preset: Option<u8>,
    pub custom_eq: Vec<CustomEq>,
    pub volume_enhancer: Option<bool>,
    pub dynamic_bands: Vec<DynamicBand>,
    pub multipoint: Vec<MultipointDevice>,
    /// Queries that failed are reported, never replaced by plausible default values.
    pub errors: Vec<String>,
}

pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or(Error::Protocol("length overflow"))?;
        let p = self
            .data
            .get(self.offset..end)
            .ok_or(Error::Protocol("truncated payload"))?;
        self.offset = end;
        Ok(p)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }
    pub fn u16(&mut self) -> Result<u16> {
        let p = self.take(2)?;
        Ok(u16::from_le_bytes([p[0], p[1]]))
    }
    pub fn end(&self) -> Result<()> {
        if self.offset == self.data.len() {
            Ok(())
        } else {
            Err(Error::Protocol("unexpected trailing data"))
        }
    }
}
pub fn parse_battery(data: &[u8]) -> Result<Vec<Battery>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        let component = c.u8()?;
        let raw = c.u8()?;
        let level = raw & 127;
        v.push(Battery {
            component,
            percent: (level <= 100).then_some(level),
            charging: raw & 128 != 0,
            raw,
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_wear(data: &[u8]) -> Result<Vec<WearState>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        let component = c.u8()?;
        let raw = c.u8()?;
        v.push(WearState {
            component,
            in_case: raw & 1 == 0,
            worn: raw & 2 != 0,
            connected: raw & 4 == 0,
            raw,
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_features(data: &[u8]) -> Result<Vec<FeatureState>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        let id = c.u8()?;
        let b = c.u8()?;
        if b > 1 {
            return Err(Error::Protocol("feature boolean"));
        }
        if id != 0 {
            v.push(FeatureState {
                id,
                enabled: b != 0,
            });
        }
    }
    c.end()?;
    Ok(v)
}
pub fn parse_touch(data: &[u8]) -> Result<Vec<TouchBinding>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        v.push(TouchBinding {
            ear: c.u8()?,
            button: c.u8()?,
            gesture: c.u8()?,
            function: c.u8()?,
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_noise(data: &[u8]) -> Result<NoiseState> {
    let mut c = Cursor::new(data);
    let n = NoiseState {
        action: c.u8()?,
        kind: c.u8()?,
        value: c.u8()?,
    };
    c.end()?;
    Ok(n)
}
pub fn parse_versions(data: &[u8]) -> Result<Vec<Version>> {
    let (&count, rest) = data
        .split_first()
        .ok_or(Error::Protocol("missing version count"))?;
    let s = std::str::from_utf8(rest).map_err(|_| Error::Protocol("version UTF-8"))?;
    let parts: Vec<_> = s.split(',').collect();
    if parts.len() != count as usize * 3 {
        return Err(Error::Protocol("version count"));
    }
    parts
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| {
            Ok(Version {
                component: p[0]
                    .parse()
                    .map_err(|_| Error::Protocol("version component"))?,
                kind: p[1].parse().map_err(|_| Error::Protocol("version kind"))?,
                value: p[2].into(),
            })
        })
        .collect()
}
pub fn parse_custom_eq(data: &[u8]) -> Result<Vec<CustomEq>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        let selected = c.u8()? != 0;
        let min = c.i8()?;
        let max = c.i8()?;
        let id = c.u8()?;
        let len = c.u8()? as usize;
        let name = std::str::from_utf8(c.take(len)?)
            .map_err(|_| Error::Protocol("EQ UTF-8"))?
            .to_owned();
        let n = c.u8()?;
        let mut bands = Vec::new();
        for _ in 0..n {
            bands.push(EqBand {
                frequency: c.u16()?,
                gain: c.i8()?,
            });
        }
        let eq = CustomEq {
            selected,
            min,
            max,
            id,
            name,
            bands,
        };
        eq.validate().map_err(|_| Error::Protocol("invalid EQ"))?;
        v.push(eq);
    }
    c.end()?;
    Ok(v)
}
pub fn parse_dynamic_band(data: &[u8]) -> Result<DynamicBand> {
    let mut c = Cursor::new(data);
    let b = DynamicBand {
        min: c.i8()?,
        max: c.i8()?,
        value: c.i8()?,
        kind: c.u8()?,
    };
    c.end()?;
    if b.min > b.max || !(b.min..=b.max).contains(&b.value) {
        return Err(Error::Protocol("dynamic band bounds"));
    }
    Ok(b)
}
pub fn parse_dynamic_bands(data: &[u8]) -> Result<Vec<DynamicBand>> {
    let mut c = Cursor::new(data);
    let n = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(parse_dynamic_band(c.take(4)?)?);
    }
    c.end()?;
    Ok(v)
}
pub fn parse_multipoint(data: &[u8]) -> Result<Vec<MultipointDevice>> {
    let mut c = Cursor::new(data);
    let count = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..count {
        let addr = c
            .take(6)?
            .iter()
            .rev()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        let len = c.u8()? as usize;
        let mut entry = Cursor::new(c.take(len)?);
        let state = entry.u8()?;
        let flags = entry.u8()?;
        let current = flags == 1;
        let n = entry.u8()? as usize;
        let name_bytes = entry.take(n)?.to_vec();
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();
        let extension = entry.data[entry.offset..].to_vec();
        v.push(MultipointDevice {
            address: addr,
            state,
            current,
            flags,
            name,
            name_bytes,
            extension,
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_event(p: Packet) -> Result<Event> {
    Ok(match p.command {
        cmd::CUSTOM_EQ_EVENT => Event::CustomEq(parse_custom_eq(&p.payload)?),
        cmd::DYNAMIC_BAND_EVENT => Event::DynamicBand(parse_dynamic_band(&p.payload)?),
        0x8129 if p.payload.first() == Some(&0) => {
            Event::StatusFeatures(super::advanced::parse_status_features(&p.payload[1..])?)
        }
        0x812a if p.payload.len() == 6 && p.payload[0] == 0 => {
            Event::FeatureDetection(super::advanced::StatusFeature {
                id: p.payload[1],
                value: u32::from_le_bytes(p.payload[2..6].try_into().expect("four bytes")),
            })
        }
        0x050e => Event::UseTime(super::advanced::parse_use_time(&p.payload)?),
        0x0502 if p.payload.len() == 1 => Event::HostOperation(p.payload[0]),
        0x0503 => Event::Features(parse_features(&p.payload)?),
        0x0504 if p.payload.len() == 1 => Event::EqPreset(p.payload[0]),
        cmd::NOTIFICATION => {
            let Some((&kind, b)) = p.payload.split_first() else {
                return Err(Error::Protocol("empty notification"));
            };
            match kind {
                1 => Event::Battery(parse_battery(b)?),
                2 => Event::Wear(parse_wear(b)?),
                3 if b.len() == 2 => Event::Noise(NoiseState {
                    action: 1,
                    kind: b[0],
                    value: b[1],
                }),
                3 if b.len() == 3 => Event::Noise(parse_noise(b)?),
                // Air7 cycle pushes include one trailing reserved zero (query replies do not).
                3 if b.len() == 4 && b[3] == 0 => Event::Noise(parse_noise(&b[..3])?),
                4 if b.len() == 4 => Event::Fit(vec![
                    FitResult {
                        ear: b[0],
                        result: b[1],
                    },
                    FitResult {
                        ear: b[2],
                        result: b[3],
                    },
                ]),
                6 => Event::Multipoint(parse_multipoint(b)?),
                0xf1 => Event::UserInteraction(parse_user_interaction(b)?),
                _ => Event::Raw(p),
            }
        }
        _ => Event::Raw(p),
    })
}
