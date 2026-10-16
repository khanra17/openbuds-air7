// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use super::{
    Error, Result,
    codec::{self, Decoder, Packet},
    commands as cmd,
    model::*,
};
use crate::Transport;
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// One outstanding request at a time. Notifications are retained separately.
/// No automatic retries: an acknowledgement can be lost after a write succeeded.
pub struct Client<T: Transport> {
    transport: T,
    decoder: Decoder,
    received: VecDeque<Packet>,
    events: VecDeque<Event>,
    transaction: u8,
    frame_size: usize,
    timeout: Duration,
    capabilities: Option<Capabilities>,
}
impl<T: Transport> Client<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            decoder: Decoder::default(),
            received: VecDeque::new(),
            events: VecDeque::new(),
            transaction: 0,
            frame_size: 1946,
            timeout: Duration::from_secs(12),
            capabilities: None,
        }
    }
    pub fn set_timeout(&mut self, t: Duration) -> Result<()> {
        if t.is_zero() {
            return Err(Error::Invalid("zero timeout"));
        }
        self.timeout = t;
        Ok(())
    }
    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.capabilities.as_ref()
    }
    pub fn into_transport(self) -> T {
        self.transport
    }
    fn ensure_supported(&self, c: u16) -> Result<()> {
        if self
            .capabilities
            .as_ref()
            .is_some_and(|cap| !cap.supports(c))
        {
            Err(Error::Unsupported(c))
        } else {
            Ok(())
        }
    }
    fn send(&mut self, p: &Packet) -> Result<()> {
        for f in codec::encode(p, self.frame_size)? {
            self.transport.write_all(&f)?;
        }
        self.transport.flush()?;
        Ok(())
    }
    fn make_request(&mut self, c: u16, payload: Vec<u8>) -> Result<Packet> {
        self.ensure_supported(c)?;
        self.make_request_unchecked(c, payload)
    }
    fn make_request_unchecked(&mut self, c: u16, payload: Vec<u8>) -> Result<Packet> {
        let p = Packet::new(c, self.transaction, payload);
        self.transaction = self.transaction.wrapping_add(1);
        self.send(&p)?;
        Ok(p)
    }
    fn receive(&mut self, deadline: Instant) -> Result<Packet> {
        loop {
            if let Some(p) = self.received.pop_front() {
                return Ok(p);
            }
            let t = deadline
                .checked_duration_since(Instant::now())
                .ok_or(Error::Timeout)?;
            self.transport.set_read_timeout(t)?;
            let mut b = [0; 4096];
            match self.transport.read(&mut b) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Bluetooth disconnected",
                    )
                    .into());
                }
                Ok(n) => self.received.extend(self.decoder.feed(&b[..n])?),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(Error::Timeout);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    fn event(&mut self, p: Packet) -> Result<Event> {
        // The earbuds occasionally ask for the host time. Reply with the same transaction.
        if p.command == 0x0500 {
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| Error::Protocol("host clock before epoch"))?
                .as_secs() as u32;
            let mut b = vec![0];
            b.extend(seconds.to_le_bytes());
            self.send(&Packet::new(0x8500, p.transaction, b))?;
        }
        parse_event(p)
    }
    fn remember(&mut self, event: Event) {
        if self.events.len() == 256 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
    fn response_payload(p: Packet, c: u16) -> Result<Vec<u8>> {
        let Some((&status, data)) = p.payload.split_first() else {
            return Err(Error::Protocol("empty response status"));
        };
        if status != 0 {
            return Err(Error::Rejected { command: c, status });
        }
        Ok(data.to_vec())
    }
    /// Diagnostic escape hatch. Do not send undocumented destructive commands.
    /// Returns the response data with its leading status byte removed.
    pub fn request(&mut self, command: u16, payload: impl Into<Vec<u8>>) -> Result<Vec<u8>> {
        self.ensure_supported(command)?;
        self.request_read_or_checked(command, payload.into())
    }
    /// Internal path for an explicitly enumerated read-only SDK property probe.
    pub(crate) fn request_read_or_checked(
        &mut self,
        command: u16,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let req = self.make_request_unchecked(command, payload)?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let p = self.receive(deadline)?;
            if p.response_to(&req) {
                return Self::response_payload(p, command);
            }
            let ev = self.event(p)?;
            self.remember(ev);
        }
    }
    /// Diagnostic reads may push multiple replies using the device's event counter instead of
    /// the request transaction. Collect only this known command until the reply stream goes quiet.
    pub(crate) fn collect_read_responses(
        &mut self,
        command: u16,
        payload: Vec<u8>,
    ) -> Result<Vec<Vec<u8>>> {
        self.make_request_unchecked(command, payload)?;
        let total = Instant::now() + self.timeout;
        let mut deadline = total;
        let mut data = Vec::new();
        loop {
            match self.receive(deadline) {
                Ok(p) if p.command == command | 0x8000 => {
                    data.push(Self::response_payload(p, command)?);
                    deadline = total.min(Instant::now() + Duration::from_millis(400));
                }
                Ok(p) => {
                    let e = self.event(p)?;
                    self.remember(e);
                }
                Err(Error::Timeout) if !data.is_empty() => return Ok(data),
                Err(e) => return Err(e),
            }
        }
    }
    /// Read and validate the device capability bitmap, then negotiate the host receive MTU.
    pub fn initialize(&mut self) -> Result<Capabilities> {
        let data = self.request(cmd::CAPABILITIES, vec![])?;
        if data.is_empty() {
            return Err(Error::Protocol("empty capabilities"));
        }
        let cap = Capabilities::parse(&data);
        self.capabilities = Some(cap.clone());
        let data = self.request(cmd::SET_HOST_MTU, vec![0, 2])?;
        if data.len() != 2 {
            return Err(Error::Protocol("peer MTU length"));
        }
        // This is the stream MTU, not the logical frame size. RFCOMM performs segmentation.
        let mtu = u16::from_le_bytes([data[0], data[1]]);
        if mtu == 0 {
            return Err(Error::Protocol("peer MTU zero"));
        }
        Ok(cap)
    }
    pub fn next_event(&mut self, timeout: Duration) -> Result<Event> {
        if let Some(e) = self.events.pop_front() {
            return Ok(e);
        }
        let p = self.receive(Instant::now() + timeout)?;
        self.event(p)
    }
    pub fn battery(&mut self) -> Result<Vec<Battery>> {
        parse_battery(&self.request(cmd::BATTERY, vec![])?)
    }
    pub fn versions(&mut self) -> Result<Vec<Version>> {
        parse_versions(&self.request(cmd::VERSIONS, vec![])?)
    }
    pub fn wear(&mut self) -> Result<Vec<WearState>> {
        parse_wear(&self.request(cmd::WEAR, vec![])?)
    }
    pub fn features(&mut self) -> Result<Vec<FeatureState>> {
        self.features_by_ids(cmd::FEATURE_IDS)
    }
    /// Query explicit feature IDs, including SDK flags without a known UI name.
    pub fn features_by_ids(&mut self, ids: &[u8]) -> Result<Vec<FeatureState>> {
        if ids.is_empty() || ids.len() > 255 {
            return Err(Error::Invalid("feature ID count"));
        }
        let mut p = vec![ids.len() as u8];
        p.extend(ids);
        parse_features(&self.request(cmd::FEATURES, p)?)
    }
    /// Advanced flag setter. Obtain the current ID/state through readback first.
    pub fn set_feature_id(&mut self, id: u8, enabled: bool) -> Result<()> {
        if id == 0 {
            return Err(Error::Invalid(
                "feature ID zero is an unsupported placeholder",
            ));
        }
        if !self.features_by_ids(&[id])?.iter().any(|f| f.id == id) {
            return Err(Error::Invalid("device did not return this feature ID"));
        }
        self.request(cmd::SET_FEATURE, vec![id, u8::from(enabled)])?;
        Ok(())
    }
    /// Sends a capability-checked command that has no standard reply. Delivery is not confirmation.
    pub fn send_unacknowledged(&mut self, command: u16, payload: impl Into<Vec<u8>>) -> Result<u8> {
        Ok(self.make_request(command, payload.into())?.transaction)
    }
    pub fn feature(&mut self, f: Feature) -> Result<bool> {
        let b = self.request(cmd::FEATURES, vec![1, f as u8])?;
        parse_features(&b)?
            .into_iter()
            .find(|v| v.id == f as u8)
            .map(|v| v.enabled)
            .ok_or(Error::Protocol("missing feature"))
    }
    pub fn set_feature(&mut self, f: Feature, enabled: bool) -> Result<()> {
        self.request(cmd::SET_FEATURE, vec![f as u8, u8::from(enabled)])?;
        Ok(())
    }
    pub fn noise(&mut self) -> Result<NoiseState> {
        parse_noise(&self.request(cmd::NOISE, vec![1, 1])?)
    }
    pub fn noise_cycle(&mut self) -> Result<NoiseState> {
        parse_noise(&self.request(cmd::NOISE, vec![2, 1])?)
    }
    pub fn set_noise(&mut self, mode: NoiseMode) -> Result<()> {
        self.request(cmd::SET_NOISE, vec![1, 1, mode as u8])?;
        Ok(())
    }
    pub fn set_noise_cycle(&mut self, cycle: NoiseCycle) -> Result<()> {
        self.request(cmd::SET_NOISE, vec![2, 1, cycle as u8])?;
        Ok(())
    }
    pub fn touch(&mut self) -> Result<Vec<TouchBinding>> {
        let mut p = vec![cmd::TOUCH_FUNCTION_IDS.len() as u8];
        p.extend(cmd::TOUCH_FUNCTION_IDS);
        parse_touch(&self.request(cmd::TOUCH, p)?)
    }
    pub fn set_touch(&mut self, bindings: &[TouchBinding]) -> Result<()> {
        if bindings.is_empty() || bindings.len() > 255 {
            return Err(Error::Invalid("invalid number of touch bindings"));
        }
        let mut p = vec![bindings.len() as u8];
        for b in bindings {
            if !matches!(b.ear, 1 | 2)
                || b.button != 1
                || !matches!(b.gesture, 1 | 2 | 3 | 4 | 6)
                || !cmd::TOUCH_FUNCTION_IDS.contains(&b.function)
            {
                return Err(Error::Invalid("touch binding outside known values"));
            }
            p.extend([b.ear, b.button, b.gesture, b.function]);
        }
        self.request(cmd::SET_TOUCH, p)?;
        Ok(())
    }
    pub fn eq_preset(&mut self) -> Result<u8> {
        let b = self.request(cmd::EQ_PRESET, vec![])?;
        if b.len() != 1 {
            return Err(Error::Protocol("EQ preset length"));
        }
        Ok(b[0])
    }
    pub fn set_eq_preset(&mut self, id: u8) -> Result<()> {
        if id > 3 {
            return Err(Error::Invalid(
                "Air7 preset must be 0..3; select custom EQ by updating it",
            ));
        }
        self.request(cmd::SET_EQ_PRESET, vec![id])?;
        Ok(())
    }
    pub fn custom_eq(&mut self) -> Result<Vec<CustomEq>> {
        parse_custom_eq(&self.request(cmd::CUSTOM_EQ, vec![])?)
    }
    /// Add/update/delete an EQ. Update also selects it; rename by updating the name.
    /// Preserve its device-provided bounds. Save the old preset before a temporary update.
    pub fn set_custom_eq(&mut self, eq: &CustomEq, action: EqAction) -> Result<()> {
        self.request(cmd::SET_CUSTOM_EQ, eq.set_payload(action)?)?;
        Ok(())
    }
    pub fn dynamic_bands(&mut self) -> Result<Vec<DynamicBand>> {
        parse_dynamic_bands(&self.request(cmd::DYNAMIC_BANDS, vec![])?)
    }
    /// Accept either the firmware-specific 0x841b acknowledgement or a 0x0508 push.
    pub fn set_dynamic_band(&mut self, band: DynamicBand) -> Result<()> {
        if !(1..=3).contains(&band.kind)
            || band.min > band.max
            || !(band.min..=band.max).contains(&band.value)
        {
            return Err(Error::Invalid("dynamic band outside limits"));
        }
        let req = self.make_request(
            cmd::SET_DYNAMIC_BAND,
            vec![band.min as u8, band.max as u8, band.value as u8, band.kind],
        )?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let p = self.receive(deadline)?;
            if p.response_to(&req) {
                let data = Self::response_payload(p, cmd::SET_DYNAMIC_BAND)?;
                // Air7 appends 01 for every band, not the requested band ID.
                // Treat it as opaque ack metadata; the following push/readback identifies the band.
                if data.len() > 1 {
                    return Err(Error::Protocol("dynamic acknowledgement length"));
                }
                return Ok(());
            }
            let event = self.event(p)?;
            let confirmed = event == Event::DynamicBand(band);
            self.remember(event);
            if confirmed {
                return Ok(());
            }
        }
    }
    pub fn volume_enhancer(&mut self) -> Result<bool> {
        let b = self.request(cmd::VOLUME_ENHANCER, vec![])?;
        if b.len() != 1 || b[0] > 1 {
            return Err(Error::Protocol("volume enhancer boolean"));
        }
        Ok(b[0] != 0)
    }
    /// Enabling can make audio louder. Caller must obtain the listener's consent.
    pub fn set_volume_enhancer(&mut self, enabled: bool) -> Result<()> {
        self.request(cmd::SET_VOLUME_ENHANCER, vec![u8::from(enabled)])?;
        Ok(())
    }
    pub fn multipoint(&mut self) -> Result<Vec<MultipointDevice>> {
        parse_multipoint(&self.request(cmd::MULTIPOINT, vec![])?)
    }
    /// Wire actions: 0 disconnect, 1 connect, 2 remove. Untested on the laptop; may disconnect it.
    pub fn multipoint_action(&mut self, address: [u8; 6], action: u8) -> Result<()> {
        if action > 2 {
            return Err(Error::Invalid("unknown multipoint action"));
        }
        let mut p = vec![1];
        p.extend(address.into_iter().rev());
        p.push(action);
        self.request(cmd::MULTIPOINT_ACTION, p)?;
        Ok(())
    }
    /// Starts/stops the fit test. Starting can play a test sound; requires worn earbuds.
    pub fn fit_test(&mut self, start: bool) -> Result<()> {
        self.request(cmd::FIT_TEST, vec![u8::from(start)])?;
        Ok(())
    }
    /// Find-earbud audio can be loud. Never enable while earbuds are worn.
    pub fn find_buds(&mut self, start: bool) -> Result<()> {
        if start {
            let ears: Vec<_> = self
                .wear()?
                .into_iter()
                .filter(|w| matches!(w.component, 1 | 2))
                .collect();
            if ears.len() != 2 || ears.iter().any(|w| w.worn || w.in_case || !w.connected) {
                return Err(Error::Invalid(
                    "find tone requires both earbuds out of case, connected and not worn",
                ));
            }
        }
        self.request(cmd::FIND_BUDS, vec![u8::from(start)])?;
        Ok(())
    }
    /// Best-effort bounded finding tone. Always sends stop, even after a failed start ack.
    /// Process termination/power loss can prevent cleanup; this is not a hardware watchdog.
    pub fn find_buds_for(&mut self, duration: Duration) -> Result<()> {
        if duration.is_zero() || duration > Duration::from_secs(10) {
            return Err(Error::Invalid("find duration must be 1..10 seconds"));
        }
        self.find_buds_for_with_cancel(duration, || false)
    }
    pub fn find_buds_for_with_cancel(
        &mut self,
        duration: Duration,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<()> {
        if duration.is_zero() || duration > Duration::from_secs(10) {
            return Err(Error::Invalid("find duration must be 1..10 seconds"));
        }
        let result = (|| {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            self.find_buds(true)?;
            let end = Instant::now() + duration;
            while Instant::now() < end {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(())
        })();
        self.find_buds(false)?;
        result
    }
    /// Run a fit test, collect its results, and stop even on timeout/rejection.
    /// At least one worn ear is required; the firmware may reject a one-ear test.
    pub fn run_fit_test(&mut self, timeout: Duration) -> Result<Vec<FitResult>> {
        self.run_fit_test_with_cancel(timeout, || false)
    }
    /// A cancellation callback is checked between bounded reads; cleanup still sends stop.
    pub fn run_fit_test_with_cancel(
        &mut self,
        timeout: Duration,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<FitResult>> {
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(Error::Invalid("fit timeout must be 1..30 seconds"));
        }
        if !self
            .wear()?
            .iter()
            .any(|w| matches!(w.component, 1 | 2) && w.worn && w.connected)
        {
            return Err(Error::Invalid(
                "fit test requires a worn earbud; app expects both",
            ));
        }
        self.subscribe()?;
        let result = (|| {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            self.events.retain(|e| !matches!(e, Event::Fit(_)));
            self.received
                .retain(|p| !(p.command == cmd::NOTIFICATION && p.payload.first() == Some(&4)));
            let request = self.make_request(cmd::FIT_TEST, vec![1])?;
            let deadline = Instant::now() + timeout;
            let mut acknowledged = false;
            let mut results = None;
            loop {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                if Instant::now() >= deadline {
                    return Err(Error::Timeout);
                }
                let p =
                    match self.receive(deadline.min(Instant::now() + Duration::from_millis(200))) {
                        Err(Error::Timeout) => continue,
                        other => other?,
                    };
                if p.response_to(&request) {
                    Self::response_payload(p, cmd::FIT_TEST)?;
                    acknowledged = true;
                } else {
                    let event = self.event(p)?;
                    if let Event::Fit(v) = event {
                        results = Some(v);
                    } else {
                        self.remember(event);
                    }
                }
                if acknowledged && let Some(v) = results.take() {
                    return Ok(v);
                }
            }
        })();
        self.fit_test(false)?;
        result
    }
    /// Register advertised notifications in a batch. Some firmware acknowledges only by pushes.
    pub fn subscribe(&mut self) -> Result<()> {
        let b = self.request(cmd::NOTIFICATION_CAPABILITIES, vec![])?;
        let mut c = Cursor::new(&b);
        let count = c.u8()?;
        let ids = c.take(count as usize)?;
        c.end()?;
        let mut p = vec![count];
        p.extend(ids);
        let _ = self.make_request(cmd::SUBSCRIBE, p)?;
        Ok(())
    }
    /// Read every supported setting used by the Buds Air7 screens, reporting partial failures.
    pub fn snapshot(&mut self) -> Snapshot {
        let mut s = Snapshot {
            capabilities: self.capabilities.clone().unwrap_or_default(),
            ..Default::default()
        };
        macro_rules! collect {
            ($field:ident,$call:expr) => {
                match $call {
                    Ok(v) => s.$field = v,
                    Err(Error::Unsupported(_)) => {}
                    Err(e) => s.errors.push(format!("{}: {e}", stringify!($field))),
                }
            };
        }
        collect!(battery, self.battery());
        collect!(versions, self.versions());
        collect!(wear, self.wear());
        collect!(features, self.features());
        collect!(noise, self.noise().map(Some));
        collect!(noise_cycle, self.noise_cycle().map(Some));
        collect!(touch, self.touch());
        collect!(eq_preset, self.eq_preset().map(Some));
        collect!(custom_eq, self.custom_eq());
        collect!(volume_enhancer, self.volume_enhancer().map(Some));
        collect!(dynamic_bands, self.dynamic_bands());
        collect!(multipoint, self.multipoint());
        s
    }
}
