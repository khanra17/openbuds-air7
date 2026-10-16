// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use ::realme_buds::devices::air7 as realme_buds;
use clap::{Parser, Subcommand, ValueEnum};
use realme_buds::{AncLevel, Feature, NoiseCycle, NoiseMode};
#[derive(Parser)]
#[command(
    name = "buds",
    version,
    about = "Unofficial realme Buds Air7 Bluetooth controller"
)]
pub(crate) struct Args {
    /// Paired Bluetooth address; defaults to the only paired Buds Air7.
    #[arg(long, global = true)]
    pub(crate) address: Option<String>,
    /// Diagnostic RFCOMM channel override. Normal use resolves the vendor UUID through SDP.
    #[arg(long, global = true)]
    pub(crate) channel: Option<u32>,
    #[arg(long, global = true, default_value_t = 12)]
    pub(crate) timeout: u64,
    /// Opt-in wire log. Can contain private device names/addresses/profile data.
    #[arg(long, global = true)]
    pub(crate) trace: Option<std::path::PathBuf>,
    /// Use the original numeric wire-level output instead of named semantic descriptions.
    #[arg(long, global = true)]
    pub(crate) raw: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    Devices,
    Status,
    Battery,
    Versions,
    /// Per-ear wearing/case/connection flags, including partial or missing ears.
    Wear,
    /// Firmware metadata and upgrade capability reads only; never flashes firmware.
    Firmware,
    /// Enumerated read-only SDK queries, including properties beyond the UI.
    Diagnostics,
    FeatureIds {
        #[arg(num_args=1..)]
        ids: Vec<u8>,
    },
    ToggleId {
        id: u8,
        #[arg(value_enum)]
        state: Switch,
    },
    Connection {
        address: String,
        #[arg(value_enum)]
        action: ConnectionArg,
        #[arg(long)]
        confirm: bool,
    },
    Noise {
        #[arg(value_enum)]
        mode: Option<NoiseArg>,
        /// Required with 'noise anc'; there is no separately readable ANC-enable bit.
        #[arg(long, value_enum)]
        level: Option<AncArg>,
    },
    /// UI-style ANC enable/disable. 'on' requires --level; 'off' selects noise Off.
    Anc {
        #[arg(value_enum)]
        state: Option<Switch>,
        #[arg(long, value_enum)]
        level: Option<AncArg>,
    },
    /// Transparency is mutually exclusive with ANC; 'off' selects noise Off.
    Transparency {
        #[arg(value_enum)]
        state: Option<Switch>,
    },
    Cycle {
        #[arg(value_enum)]
        mode: Option<CycleArg>,
    },
    Features,
    Toggle {
        #[arg(value_enum)]
        feature: FeatureArg,
        #[arg(value_enum)]
        state: Switch,
    },
    EqList,
    EqPreset {
        id: Option<u8>,
    },
    /// Six gains for 62,250,1000,4000,8000,16000 Hz, each -6..6 dB.
    EqSave {
        id: u8,
        name: String,
        #[arg(num_args = 6, allow_hyphen_values = true)]
        gains: Vec<i8>,
        #[arg(long)]
        add: bool,
    },
    EqSelect {
        id: u8,
    },
    EqRename {
        id: u8,
        name: String,
    },
    EqDelete {
        id: u8,
        #[arg(long)]
        confirm: bool,
    },
    Dynamic {
        band: Option<u8>,
        #[arg(allow_hyphen_values = true)]
        value: Option<i8>,
    },
    Touch,
    /// Wire IDs documented in docs/protocol.md: ear 1 = left, 2 = right.
    Bind {
        ear: u8,
        gesture: u8,
        function: u8,
    },
    Volume {
        #[arg(value_enum)]
        state: Option<Switch>,
        #[arg(long)]
        confirm_louder: bool,
    },
    Multipoint,
    /// Stream notifications until Ctrl+C.
    Watch,
    /// Never starts if wear flags are unknown/worn. Tone is automatically stopped after 3 seconds.
    Find {
        #[arg(value_enum)]
        state: Switch,
        #[arg(long)]
        confirm_sound: bool,
    },
    /// Starting the fit test plays a sound and requires wearing the earbuds.
    Fit {
        #[arg(value_enum)]
        state: Switch,
        #[arg(long)]
        confirm_sound: bool,
    },
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ConnectionArg {
    Disconnect,
    Connect,
    Remove,
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Switch {
    On,
    Off,
}
impl Switch {
    pub(crate) fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum NoiseArg {
    Off,
    Transparency,
    Mild,
    Moderate,
    Maximum,
    Adaptive,
    Anc,
}
impl NoiseArg {
    pub(crate) fn mode(self, level: Option<AncArg>) -> realme_buds::Result<NoiseMode> {
        if matches!(self, Self::Anc) {
            return level
                .map(|v| AncLevel::from(v).mode())
                .ok_or(realme_buds::Error::Invalid("noise anc requires --level"));
        }
        if level.is_some() {
            return Err(realme_buds::Error::Invalid("--level is only for noise anc"));
        }
        Ok(match self {
            Self::Off => NoiseMode::Off,
            Self::Transparency => NoiseMode::Transparency,
            Self::Mild => NoiseMode::Mild,
            Self::Moderate => NoiseMode::Moderate,
            Self::Maximum => NoiseMode::Maximum,
            Self::Adaptive => NoiseMode::Adaptive,
            Self::Anc => unreachable!(),
        })
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AncArg {
    Mild,
    Moderate,
    Maximum,
    Adaptive,
}
impl From<AncArg> for AncLevel {
    fn from(v: AncArg) -> Self {
        match v {
            AncArg::Mild => Self::Mild,
            AncArg::Moderate => Self::Moderate,
            AncArg::Maximum => Self::Maximum,
            AncArg::Adaptive => Self::Adaptive,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CycleArg {
    All,
    OffTransparency,
    OffCancellation,
    CancellationTransparency,
}
impl From<CycleArg> for NoiseCycle {
    fn from(v: CycleArg) -> Self {
        match v {
            CycleArg::All => Self::All,
            CycleArg::OffTransparency => Self::OffTransparency,
            CycleArg::OffCancellation => Self::OffCancellation,
            CycleArg::CancellationTransparency => Self::CancellationTransparency,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum FeatureArg {
    AutoPlayPause,
    AutoAnswer,
    Game,
    DualDevice,
    Lhdc,
    WindNoise,
    Spatial,
    Dynamic,
    FindPhoneGesture,
}
impl From<FeatureArg> for Feature {
    fn from(v: FeatureArg) -> Self {
        match v {
            FeatureArg::AutoPlayPause => Self::AutoPlayPause,
            FeatureArg::AutoAnswer => Self::AutoAnswer,
            FeatureArg::Game => Self::GameMode,
            FeatureArg::DualDevice => Self::DualDevice,
            FeatureArg::Lhdc => Self::HighDefinitionSound,
            FeatureArg::WindNoise => Self::WindNoiseReduction,
            FeatureArg::Spatial => Self::SpatialAudio,
            FeatureArg::Dynamic => Self::DynamicAudio,
            FeatureArg::FindPhoneGesture => Self::FindPhoneGesture,
        }
    }
}
