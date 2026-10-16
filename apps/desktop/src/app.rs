// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use crate::worker::{Action, Reply, start_worker};
use ::realme_buds::devices::air7 as realme_buds;
use eframe::egui;
use realme_buds::{
    AncLevel, AncPreference, Component, CustomEq, EqAction, Event, Feature, NoiseCategory,
    NoiseCycle, NoiseMode, Snapshot,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    time::Duration,
};

pub(crate) struct App {
    tx: Sender<Action>,
    rx: Receiver<Reply>,
    devices: Vec<(String, String)>,
    address: String,
    state: Snapshot,
    connected: bool,
    busy: bool,
    status: String,
    error: bool,
    editor: CustomEq,
    new_eq: bool,
    confirmation: Option<(String, Action)>,
    events: Vec<String>,
    diagnostics: Vec<realme_buds::advanced::PropertyResult>,
    firmware: Option<realme_buds::advanced::FirmwareInfo>,
    cancel: Arc<AtomicBool>,
    sound_active: bool,
    fit_results: Vec<realme_buds::FitResult>,
    anc_preferences: BTreeMap<String, AncPreference>,
}
impl App {
    pub(crate) fn new() -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = start_worker(cancel.clone());
        Self {
            tx,
            rx,
            devices: Vec::new(),
            address: String::new(),
            state: Snapshot::default(),
            connected: false,
            busy: false,
            status: "Pair the Air7 in Windows Settings, then connect.".into(),
            error: false,
            editor: CustomEq::air7(0, "Example EQ", [0; 6]),
            new_eq: true,
            confirmation: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
            firmware: None,
            cancel,
            sound_active: false,
            fit_results: Vec::new(),
            anc_preferences: BTreeMap::new(),
        }
    }
    fn send(&mut self, a: Action) {
        if matches!(
            a,
            Action::Fit(true) | Action::Connect(_) | Action::Disconnect
        ) {
            self.fit_results.clear();
            self.firmware = None;
            self.diagnostics.clear();
        }
        if matches!(a, Action::Fit(true) | Action::Find(true)) {
            self.cancel.store(false, Ordering::Relaxed);
            self.sound_active = true;
        }
        if self.tx.send(a).is_ok() {
            self.busy = true;
            self.error = false;
            self.status = "Working…".into();
        } else {
            self.error = true;
            self.status = "Bluetooth worker stopped".into();
        }
    }
    fn confirm(&mut self, text: &str, a: Action) {
        self.confirmation = Some((text.into(), a));
    }
    fn event(&mut self, e: Event) {
        match &e {
            Event::Fit(f) => self.fit_results = f.clone(),
            Event::Battery(b) => self.state.battery = b.clone(),
            Event::Wear(b) => self.state.wear = b.clone(),
            Event::Features(fs) => {
                for f in fs {
                    if let Some(old) = self.state.features.iter_mut().find(|x| x.id == f.id) {
                        *old = *f;
                    }
                }
            }
            Event::Noise(n) => {
                if n.action == 1 {
                    self.anc_preferences
                        .entry(self.address.clone())
                        .or_default()
                        .observe(*n);
                    self.state.noise = Some(*n);
                } else if n.action == 2 {
                    self.state.noise_cycle = Some(*n);
                }
            }
            Event::DynamicBand(b) => {
                if let Some(old) = self
                    .state
                    .dynamic_bands
                    .iter_mut()
                    .find(|x| x.kind == b.kind)
                {
                    *old = *b;
                }
            }
            Event::CustomEq(eq) => self.state.custom_eq = eq.clone(),
            Event::EqPreset(id) => self.state.eq_preset = Some(*id),
            Event::Multipoint(d) => self.state.multipoint = d.clone(),
            _ => {}
        }
        if !matches!(e, Event::Raw(_)) {
            self.events.push(format!("{:?}", e.describe()));
            if self.events.len() > 50 {
                self.events.remove(0);
            }
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Noise control");
        let noise = self.state.noise.map(|n| n.describe());
        let category = noise.as_ref().and_then(|n| n.category);
        let selected_level = self
            .anc_preferences
            .entry(self.address.clone())
            .or_default()
            .preferred_level;
        if noise.as_ref().is_none_or(|n| n.category.is_none()) {
            ui.label(
                "Current noise mode not known; see read errors/raw data. It is not assumed Off.",
            );
        }
        match self.state.wearing_summary().scenario {
            realme_buds::WearingScenario::Neither => {
                ui.label("Neither earbud is reported worn. Noise-mode writes returned firmware status 14 in that setup.");
            }
            realme_buds::WearingScenario::Unknown => {
                ui.label(
                    "Wearing observations are missing/partial; do not assume neither ear is worn.",
                );
            }
            _ => {}
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(category == Some(NoiseCategory::Off), "Off")
                .clicked()
            {
                self.send(Action::Noise(NoiseMode::Off));
            }
            if ui
                .selectable_label(
                    category == Some(NoiseCategory::Transparency),
                    "Transparency",
                )
                .clicked()
            {
                self.send(Action::Noise(NoiseMode::Transparency));
            }
            if ui
                .add_enabled(
                    selected_level.is_some(),
                    egui::Button::new("Noise cancellation (ANC)")
                        .selected(category == Some(NoiseCategory::Anc)),
                )
                .clicked()
                && let Some(level) = selected_level
            {
                self.send(Action::Anc(level));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(if category == Some(NoiseCategory::Anc) {
                "Active ANC level:"
            } else {
                "ANC level to use when enabled (local preference):"
            });
            for level in AncLevel::ALL {
                if ui
                    .selectable_label(
                        if category == Some(NoiseCategory::Anc) {
                            noise.as_ref().and_then(|n| n.active_anc_level) == Some(level)
                        } else {
                            selected_level == Some(level)
                        },
                        level.name(),
                    )
                    .clicked()
                {
                    self.anc_preferences
                        .entry(self.address.clone())
                        .or_default()
                        .choose(level);
                    if category == Some(NoiseCategory::Anc) {
                        self.send(Action::Anc(level));
                    } else {
                        self.status =
                            "ANC level chosen locally; click Noise cancellation to enable it."
                                .into();
                    }
                }
            }
        });
        ui.small("Off, Transparency and ANC are mutually exclusive. ANC uses one wire mode value that includes its level; there is no separate enable bit. Remembered level is session-only UI state.");
        ui.horizontal_wrapped(|ui| {
            ui.label("Touch-and-hold noise cycle:");
            for (name, m) in [
                ("All", NoiseCycle::All),
                ("Off / transparency", NoiseCycle::OffTransparency),
                ("Off / ANC", NoiseCycle::OffCancellation),
                ("ANC / transparency", NoiseCycle::CancellationTransparency),
            ] {
                if ui
                    .selectable_label(
                        self.state.noise_cycle.is_some_and(|s| s.value == m as u8),
                        name,
                    )
                    .clicked()
                {
                    self.send(Action::Cycle(m));
                }
            }
        });
        ui.separator();
        ui.heading("Sound and behavior");
        for &(f, label) in Feature::AIR7 {
            if let Some(value) = self
                .state
                .features
                .iter()
                .find(|v| v.id == f as u8)
                .map(|v| v.enabled)
            {
                let mut enabled = value;
                if ui.checkbox(&mut enabled, label).changed() {
                    if f == Feature::FindPhoneGesture && enabled {
                        self.confirm("Enable four-tap find-phone requests? Ringing requires a compatible host implementation; this desktop app only exposes the request event.",Action::Feature(f,enabled));
                    } else if f == Feature::DualDevice && !enabled {
                        self.confirm("Disable dual-device connection? This can disconnect the laptop or phone.",Action::Feature(f,enabled));
                    } else {
                        self.send(Action::Feature(f, enabled));
                    }
                }
            } else {
                ui.label(format!("{label}: not reported (unknown, not Off)"));
            }
        }
        if let Some(mut enabled) = self.state.volume_enhancer
            && ui.checkbox(&mut enabled, "Volume enhancer").changed()
        {
            if enabled {
                self.confirm(
                    "This can increase listening volume. Enable volume enhancer?",
                    Action::Volume(true),
                );
            } else {
                self.send(Action::Volume(false));
            }
        }
        if self.state.volume_enhancer.is_none() {
            ui.label("Volume enhancer: not reported (unknown, not Off)");
        }
        ui.small("LHDC enables an earbud preference; actual codec negotiation depends on the host. Find-phone gesture enable is stored on the earbuds, but this app does not ring a laptop speaker.");
        {
            ui.label("Stored dynamic audio bands");
            ui.small("These settings remain readable/writable when Dynamic audio is Off. The enable switch controls processing; it does not erase the bands.");
            for mut b in self.state.dynamic_bands.clone() {
                ui.horizontal(|ui| {
                    ui.label(realme_buds::band_identifier(b.kind).name);
                    if b.min <= b.max {
                        ui.add(egui::Slider::new(&mut b.value, b.min..=b.max));
                    } else {
                        ui.label(format!(
                            "Invalid reported bounds {}..{}; raw value {}",
                            b.min, b.max, b.value
                        ));
                    }
                    if ui
                        .add_enabled(
                            matches!(b.kind, 1..=3) && b.min <= b.max,
                            egui::Button::new("Apply"),
                        )
                        .clicked()
                    {
                        self.send(Action::Dynamic(b));
                    }
                });
            }
        }
        ui.separator();
        ui.heading("Equalizer");
        if let realme_buds::SoundSelection::ConflictingCustomSelections(ids) =
            self.state.sound_selection()
        {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("Firmware reports conflicting selected custom EQs: {ids:?}"),
            );
        }
        ui.horizontal_wrapped(|ui| {
            for (id, label) in [
                (0, "Original sound"),
                (1, "Serenade"),
                (2, "Clear Bass"),
                (3, "Deep Bass"),
            ] {
                if ui
                    .selectable_label(
                        self.state.eq_preset == Some(id)
                            && !self.state.custom_eq.iter().any(|e| e.selected),
                        label,
                    )
                    .clicked()
                {
                    self.send(Action::Preset(id));
                }
            }
        });
        for eq in self.state.custom_eq.clone() {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} (#{}){}",
                    eq.name,
                    eq.id,
                    if eq.selected { " — selected" } else { "" }
                ));
                if ui.button("Select").clicked() {
                    self.send(Action::Eq(eq.clone(), EqAction::Update));
                }
                if ui.button("Edit / rename").clicked() {
                    self.editor = eq.clone();
                    self.new_eq = false;
                }
                if ui.button("Delete").clicked() {
                    self.confirm("Delete this custom EQ?", Action::Eq(eq, EqAction::Delete));
                }
            });
        }
        if ui.button("New custom EQ").clicked() {
            self.editor = CustomEq::air7(0, "My EQ", [0; 6]);
            self.new_eq = true;
        }
        ui.group(|ui| {
            ui.label(if self.new_eq {
                "New EQ (the earbuds assign its ID)"
            } else {
                "Edit EQ (saving also selects it)"
            });
            ui.text_edit_singleline(&mut self.editor.name);
            for b in &mut self.editor.bands {
                ui.horizontal(|ui| {
                    ui.label(format!("{} Hz", b.frequency));
                    ui.add(
                        egui::Slider::new(&mut b.gain, self.editor.min..=self.editor.max)
                            .suffix(" dB"),
                    );
                });
            }
            if ui.button("Save to earbuds").clicked() {
                match self.editor.validate() {
                    Ok(()) => self.send(Action::Eq(
                        self.editor.clone(),
                        if self.new_eq {
                            EqAction::Add
                        } else {
                            EqAction::Update
                        },
                    )),
                    Err(e) => {
                        self.error = true;
                        self.status = e.to_string();
                    }
                }
            }
        });
        ui.separator();
        ui.heading("Touch bindings");
        ui.small(
            "All returned bindings are shown, including Single tap and Long hold. SDK-listed choices are not proof the physical gesture is implemented on Air7; unidentified IDs remain explicit.",
        );
        for mut b in self.state.touch.clone().into_iter() {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} (#{} ) · {}",
                    Component::describe(b.ear).name,
                    b.ear,
                    realme_buds::gesture_identifier(b.gesture).name
                ));
                egui::ComboBox::from_id_salt((b.ear, b.button, b.gesture))
                    .selected_text(function_name(b.function))
                    .show_ui(ui, |ui| {
                        for &id in realme_buds::commands::TOUCH_FUNCTION_IDS {
                            ui.selectable_value(&mut b.function, id, function_name(id));
                        }
                    });
                if ui
                    .add_enabled(
                        matches!(b.ear, 1 | 2)
                            && b.button == 1
                            && matches!(b.gesture, 1 | 2 | 3 | 4 | 6),
                        egui::Button::new("Apply"),
                    )
                    .clicked()
                {
                    self.send(Action::Bind(b));
                }
            });
        }
        ui.separator();
        ui.heading("Dual-device connections");
        for d in self.state.multipoint.clone() {
            ui.horizontal_wrapped(|ui|{
            ui.label(format!("{} · {} · {}{}",d.name,d.address,realme_buds::multipoint_state_identifier(d.state).name,if d.current{" (SDK my-phone flag; not control-socket ownership)"}else{""}));
            let addr=parse_mac(&d.address);
            if let Some(a)=addr {for(label,action)in[("Connect",1),("Disconnect",0),("Remove pairing",2)]{if ui.button(label).clicked(){self.confirm("Change this device's multipoint connection? This may disconnect the control channel. Removal loses its pairing.",Action::Multipoint(a,action));}}}
        });
        }
        ui.separator();
        ui.heading("Tests and finding");
        ui.small("Fit test works with both worn; one-ear tests return status 14. Find tone is guarded and auto-stopped. Firmware writes are not exposed.");
        ui.horizontal_wrapped(|ui|{
            if ui.button("Start fit test").clicked(){self.confirm("The fit test plays a sound. Wear the earbuds and use a quiet environment. Start?",Action::Fit(true));}
            if ui.button("Stop fit test").clicked(){self.send(Action::Fit(false));}
            if ui.button("Find earbuds").clicked(){self.confirm("Remove the earbuds from your ears first. The find tone can be loud. Start?",Action::Find(true));}
            if ui.button("Stop find tone").clicked(){self.send(Action::Find(false));}
        });
        for f in &self.fit_results {
            let ear = match f.ear {
                1 => "Left",
                2 => "Right",
                _ => "Unknown component",
            };
            ui.label(format!(
                "Last fit test · {ear} (#{}): {:?} (raw {})",
                f.ear,
                f.outcome(),
                f.result
            ));
        }
        ui.collapsing("Device information", |ui| {
            ui.label(format!(
                "Wearing: {:?}",
                self.state.wearing_summary().scenario
            ));
            if ui
                .button("Read firmware + under-the-hood diagnostics (read-only)")
                .clicked()
            {
                self.send(Action::Audit);
            }
            if let Some(f) = &self.firmware {
                ui.label(format!(
                    "Vendor {}, product {}, protocol bytes {:?}",
                    f.describe().vendor_id_hex.unwrap_or_else(||"not reported".into()),f.describe().product_id_hex.unwrap_or_else(||"not reported".into()), f.protocol_version
                ));
            }
            for r in &self.diagnostics {
                ui.label(format!(
                    "{:?} (0x{:04x}): {}",
                    r.property,
                    r.command,
                    r.describe().value.as_ref().map(|v|format!("{v:?}")).unwrap_or_else(||r.error.clone().unwrap_or_else(||"No interpreted value; inspect raw diagnostics".into()))
                ));
            }
            for v in &self.state.versions {
                ui.label(format!(
                    "{} · {}: {}",
                    Component::describe(v.component).name, realme_buds::version_identifier(v.kind).name, v.value
                ));
            }
            for w in realme_buds::describe_wear(&self.state.wear) {
                if let Some(raw)=w.raw {
                    if matches!(w.component.id,1|2){ui.label(format!("{}: worn {}, in case {}, connection flag {}",w.component.name,raw.worn,raw.in_case,raw.connected));}
                    else {ui.label(format!("{}: connection flag {}; no identified wearing/lid interpretation (raw {:02x})",w.component.name,raw.connected,raw.raw));}
                } else {ui.label(format!("{}: wear observation not returned; state unknown",w.component.name));}
            }
            ui.small("Case wearing/in-case fields have no identified lid interpretation. No observation is not an Off/0% value. Reported values may be cached inside the earbuds.");
            ui.collapsing("Additional SDK flags (read-only; effects not verified on Air7)",|ui|{
                for f in &self.state.features {if Feature::from_id(f.id).is_none(){ui.label(format!("{}: {}",realme_buds::feature_identifier(f.id).name,f.enabled));}}
            });
            ui.collapsing("Host permissions / UI-only behavior",|ui|{
                ui.label("Windows Bluetooth must be available and the Air7 paired. Phone Bluetooth/scan/location permissions, privacy consent, and platform codec availability are host prerequisites, not earbud feature bits.");
                ui.label("The remembered ANC level, unsaved EQ edits and confirmation dialogs are local UI state. See docs/state-model.md for each feature's transport/UI boundary and unresolved cases.");
            });
            if ui.button("Copy JSON status").clicked()
                && let Ok(s) = serde_json::to_string_pretty(&self.state.describe())
            {
                ui.ctx().copy_text(s);
            }
            ui.small(
                "JSON can contain Bluetooth addresses and device names. Redact before sharing.",
            );
        });
        ui.collapsing("Events / fit results", |ui| {
            for e in self.events.iter().rev() {
                ui.label(e);
            }
        });
        for e in &self.state.errors {
            ui.colored_label(egui::Color32::LIGHT_RED, e);
        }
        ui.separator();
        ui.small("Unofficial. MindFlow content, cloud services, OTA updates, and phone-side ringing are not reproduced. See docs/coverage.md.");
    }
}
fn parse_mac(s: &str) -> Option<[u8; 6]> {
    let v: Vec<_> = s
        .split(':')
        .map(|s| u8::from_str_radix(s, 16))
        .collect::<Result<_, _>>()
        .ok()?;
    v.try_into().ok()
}
fn function_name(id: u8) -> String {
    realme_buds::touch_function_identifier(id).name
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(r) = self.rx.try_recv() {
            match r {
                Reply::Devices(d) => {
                    if self.address.is_empty() {
                        self.address = d
                            .iter()
                            .find(|(n, _)| n == "realme Buds Air7")
                            .map(|(_, a)| a.clone())
                            .unwrap_or_default();
                    }
                    self.devices = d;
                }
                Reply::Connected(c) => {
                    self.connected = c;
                    if !c {
                        self.state = Snapshot::default();
                    }
                }
                Reply::State(s) => {
                    if let Some(n) = s.noise {
                        self.anc_preferences
                            .entry(self.address.clone())
                            .or_default()
                            .observe(n);
                    }
                    self.state = *s;
                }
                Reply::Event(e) => self.event(e),
                Reply::Diagnostics(d) => self.diagnostics = d,
                Reply::Firmware(f) => self.firmware = Some(f),
                Reply::Done(s) => {
                    self.sound_active = false;
                    self.busy = false;
                    self.error = false;
                    self.status = s;
                }
                Reply::Error(e) => {
                    self.sound_active = false;
                    self.busy = false;
                    self.error = true;
                    self.status = e;
                }
            }
        }
        if self.sound_active && ctx.input(|i| i.viewport().close_requested()) {
            self.cancel.store(true, Ordering::Relaxed);
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.status = "Stopping sound/test before closing; close again once stopped.".into();
        }
        egui::TopBottomPanel::top("connection").show(ctx, |ui| {
            ui.heading("OpenBuds Air7");
            ui.small("Independent controller for realme Buds Air7");
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(!self.connected && !self.busy, |ui| {
                    egui::ComboBox::from_id_salt("devices")
                        .selected_text(if self.address.is_empty() {
                            "Paired devices"
                        } else {
                            &self.address
                        })
                        .show_ui(ui, |ui| {
                            for (n, a) in &self.devices {
                                ui.selectable_value(
                                    &mut self.address,
                                    a.clone(),
                                    format!("{n} — {a}"),
                                );
                            }
                        });
                    ui.text_edit_singleline(&mut self.address);
                });
                if ui
                    .add_enabled(!self.busy && !self.connected, egui::Button::new("Connect"))
                    .clicked()
                {
                    self.send(Action::Connect(self.address.clone()));
                }
                if ui
                    .add_enabled(
                        self.connected && !self.busy,
                        egui::Button::new("Disconnect"),
                    )
                    .clicked()
                {
                    self.send(Action::Disconnect);
                }
                if ui
                    .add_enabled(self.connected && !self.busy, egui::Button::new("Refresh"))
                    .clicked()
                {
                    self.send(Action::Refresh);
                }
            });
            if self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, &self.status);
            } else {
                ui.label(&self.status);
            }
            if self.sound_active && ui.button("Stop current sound/test now").clicked() {
                self.cancel.store(true, Ordering::Relaxed);
                self.status = "Stopping…".into();
            }
            ui.horizontal(|ui| {
                for (component, label) in [
                    (realme_buds::Component::Left, "Left earbud"),
                    (realme_buds::Component::Right, "Right earbud"),
                    (realme_buds::Component::Case, "Charging case"),
                ] {
                    let text = self
                        .state
                        .battery_for(component)
                        .map(|b| {
                            let level = b
                                .percent
                                .map(|p| format!("{p}% reported"))
                                .unwrap_or_else(|| "unavailable".into());
                            format!(
                                "{label}: {level}{}",
                                if b.charging { " (charging)" } else { "" }
                            )
                        })
                        .unwrap_or_else(|| format!("{label}: unavailable"));
                    ui.label(text);
                }
                if self.busy {
                    ui.spinner();
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_enabled_ui(self.connected && !self.busy, |ui| self.controls(ui));
            });
        });
        if let Some((message, action)) = self.confirmation.clone() {
            egui::Window::new("Confirm")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirmation = None;
                        }
                        if ui.button("Confirm").clicked() {
                            self.confirmation = None;
                            self.send(action);
                        }
                    });
                });
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
