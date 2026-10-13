// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Enumerated read-only SDK diagnostics, including properties beyond the Air7 UI.
//! A capability bit is not proof a query works. Probe results retain rejection/timeout evidence.
use crate::{
    Client, Error, Result, Transport,
    model::{Cursor, Version, parse_versions},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadProperty {
    VendorId,
    ProductId,
    ProtocolVersion,
    FirmwareVersions,
    UpgradeCapabilities,
    Color,
    AutoPowerOff,
    HearingProfile,
    RestoreData,
    PersonalizedAncExists,
    EarScan,
    Language,
    CustomEq,
    BassEngine,
    UseTime,
    DynamicBands,
    NoiseAdaptiveLevel,
    FeatureStatus,
}
impl ReadProperty {
    pub const ALL: &'static [Self] = &[
        Self::VendorId,
        Self::ProductId,
        Self::ProtocolVersion,
        Self::FirmwareVersions,
        Self::UpgradeCapabilities,
        Self::Color,
        Self::AutoPowerOff,
        Self::HearingProfile,
        Self::RestoreData,
        Self::PersonalizedAncExists,
        Self::EarScan,
        Self::Language,
        Self::CustomEq,
        Self::BassEngine,
        Self::UseTime,
        Self::DynamicBands,
        Self::NoiseAdaptiveLevel,
        Self::FeatureStatus,
    ];
    pub fn command(self) -> u16 {
        match self {
            Self::VendorId => 0x0102,
            Self::ProductId => 0x0103,
            Self::ProtocolVersion => 0x0104,
            Self::FirmwareVersions => 0x0105,
            Self::UpgradeCapabilities => 0x0107,
            Self::Color => 0x010b,
            Self::AutoPowerOff => 0x0111,
            Self::HearingProfile => 0x0115,
            Self::RestoreData => 0x0118,
            Self::PersonalizedAncExists => 0x011a,
            Self::EarScan => 0x011e,
            Self::Language => 0x0120,
            Self::CustomEq => 0x0122,
            Self::BassEngine => 0x0124,
            Self::UseTime => 0x0128,
            Self::DynamicBands => 0x012c,
            Self::NoiseAdaptiveLevel => 0x010c,
            Self::FeatureStatus => 0x0129,
        }
    }
    pub fn payload(self) -> Vec<u8> {
        match self {
            // The vendor-ID exchange also tells the peer our maximum receive frame size (SDK uses 1946).
            Self::VendorId => 1946u16.to_le_bytes().to_vec(),
            Self::RestoreData => vec![1, 1],
            Self::NoiseAdaptiveLevel => vec![4, 1],
            Self::FeatureStatus => vec![10, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            _ => vec![],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyResult {
    pub property: ReadProperty,
    pub command: u16,
    pub advertised: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpgradeCapability {
    pub component: u8,
    pub supported: bool,
    pub upgrade_type: u8,
    pub raw: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusFeature {
    pub id: u8,
    pub value: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredData {
    pub id: u8,
    pub data: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UseTime {
    pub id: u8,
    pub value: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareInfo {
    pub versions: Vec<Version>,
    pub upgrade_capabilities: Vec<UpgradeCapability>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u32>,
    pub protocol_version: Vec<u8>,
    pub errors: Vec<String>,
}
pub fn parse_upgrade_capabilities(data: &[u8]) -> Result<Vec<UpgradeCapability>> {
    let mut c = Cursor::new(data);
    let n = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..n {
        let component = c.u8()?;
        let raw = c.u8()?;
        v.push(UpgradeCapability {
            component,
            supported: raw & 1 != 0,
            upgrade_type: u8::from(raw & 2 != 0),
            raw,
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_status_features(data: &[u8]) -> Result<Vec<StatusFeature>> {
    let mut c = Cursor::new(data);
    let n = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..n {
        let id = c.u8()?;
        let p = c.take(4)?;
        let value = u32::from_le_bytes(p.try_into().expect("four bytes"));
        v.push(StatusFeature { id, value });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_stored_data(data: &[u8]) -> Result<Vec<StoredData>> {
    let mut c = Cursor::new(data);
    let n = c.u8()?;
    let mut v = Vec::new();
    for _ in 0..n {
        let id = c.u8()?;
        let len = c.u16()? as usize;
        v.push(StoredData {
            id,
            data: c.take(len)?.to_vec(),
        });
    }
    c.end()?;
    Ok(v)
}
pub fn parse_use_time(data: &[u8]) -> Result<Vec<UseTime>> {
    Ok(parse_status_features(data)?
        .into_iter()
        .map(|s| UseTime {
            id: s.id,
            value: s.value,
        })
        .collect())
}
impl<T: Transport> Client<T> {
    /// Probe only a known read-only SDK property. Some queries are not in the capability bitmap.
    /// Unknown/unsupported results remain errors; opaque profile bytes are returned without guessing.
    /// This never enters the OTA command family or writes a firmware image.
    pub fn read_property(&mut self, p: ReadProperty) -> Result<Vec<u8>> {
        if p == ReadProperty::FeatureStatus {
            let status = self.status_features(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])?;
            let mut data = vec![status.len() as u8];
            for s in status {
                data.push(s.id);
                data.extend(s.value.to_le_bytes());
            }
            return Ok(data);
        }
        self.request_read_or_checked(p.command(), p.payload())
    }
    pub fn property_audit(&mut self) -> Vec<PropertyResult> {
        ReadProperty::ALL
            .iter()
            .map(|&property| {
                let command = property.command();
                let advertised = self.capabilities().is_some_and(|c| c.supports(command));
                let result = self.read_property(property);
                PropertyResult {
                    property,
                    command,
                    advertised,
                    data: result.as_ref().ok().cloned(),
                    error: result.err().map(|e| e.to_string()),
                }
            })
            .collect()
    }
    pub fn firmware_info(&mut self) -> FirmwareInfo {
        let mut f = FirmwareInfo {
            versions: Vec::new(),
            upgrade_capabilities: Vec::new(),
            vendor_id: None,
            product_id: None,
            protocol_version: Vec::new(),
            errors: Vec::new(),
        };
        match self
            .read_property(ReadProperty::FirmwareVersions)
            .and_then(|b| parse_versions(&b))
        {
            Ok(v) => f.versions = v,
            Err(e) => f.errors.push(format!("versions: {e}")),
        }
        match self
            .read_property(ReadProperty::UpgradeCapabilities)
            .and_then(|b| parse_upgrade_capabilities(&b))
        {
            Ok(v) => f.upgrade_capabilities = v,
            Err(e) => f.errors.push(format!("upgrade capabilities: {e}")),
        }
        match self.read_property(ReadProperty::VendorId) {
            Ok(b) if b.len() == 2 => f.vendor_id = Some(u16::from_le_bytes([b[0], b[1]])),
            Ok(_) => f.errors.push("vendor ID length".into()),
            Err(e) => f.errors.push(format!("vendor ID: {e}")),
        }
        match self.read_property(ReadProperty::ProductId) {
            Ok(b) if b.len() == 3 => f.product_id = Some(u32::from_le_bytes([b[0], b[1], b[2], 0])),
            Ok(_) => f.errors.push("product ID length".into()),
            Err(e) => f.errors.push(format!("product ID: {e}")),
        }
        match self.read_property(ReadProperty::ProtocolVersion) {
            Ok(b) => f.protocol_version = b,
            Err(e) => f.errors.push(format!("protocol version: {e}")),
        }
        f
    }
    pub fn status_features(&mut self, ids: &[u8]) -> Result<Vec<StatusFeature>> {
        if ids.is_empty() || ids.len() > 255 {
            return Err(Error::Invalid("status feature ID count"));
        }
        let mut p = vec![ids.len() as u8];
        p.extend(ids);
        let replies = self.collect_read_responses(0x0129, p)?;
        let mut by_id = std::collections::BTreeMap::new();
        for reply in replies {
            for s in parse_status_features(&reply)? {
                by_id.insert(s.id, s);
            }
        }
        Ok(by_id.into_values().collect())
    }
    pub fn stored_data(&mut self, ids: &[u8]) -> Result<Vec<StoredData>> {
        if ids.is_empty() || ids.len() > 255 {
            return Err(Error::Invalid("stored data ID count"));
        }
        let mut p = vec![ids.len() as u8];
        p.extend(ids);
        parse_stored_data(&self.request(0x0118, p)?)
    }
    pub fn use_time(&mut self) -> Result<Vec<UseTime>> {
        parse_use_time(&self.request(0x0128, vec![])?)
    }
    /// Advertised-model setter; the Air7 bitmap does not necessarily enable auto-power-off.
    pub fn set_auto_power_off(&mut self, minutes: u8) -> Result<()> {
        if ![0, 5, 10, 15, 20, 25, 30].contains(&minutes) {
            return Err(Error::Invalid("auto-power-off minutes"));
        }
        self.request(0x040a, vec![minutes])?;
        Ok(())
    }
}
