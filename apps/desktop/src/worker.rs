// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
use ::realme_buds::devices::air7 as realme_buds;
use realme_buds::{
    AncLevel, CustomEq, DynamicBand, EqAction, Event, Feature, NoiseCycle, NoiseMode, Snapshot,
    TouchBinding,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

#[derive(Clone)]
pub(crate) enum Action {
    Connect(String),
    Disconnect,
    Refresh,
    Audit,
    Noise(NoiseMode),
    Anc(AncLevel),
    Cycle(NoiseCycle),
    Feature(Feature, bool),
    Volume(bool),
    Preset(u8),
    Eq(CustomEq, EqAction),
    Dynamic(DynamicBand),
    Bind(TouchBinding),
    Multipoint([u8; 6], u8),
    Fit(bool),
    Find(bool),
}
pub(crate) enum Reply {
    Devices(Vec<(String, String)>),
    State(Box<Snapshot>),
    Event(Event),
    Diagnostics(Vec<realme_buds::advanced::PropertyResult>),
    Firmware(realme_buds::advanced::FirmwareInfo),
    Connected(bool),
    Done(String),
    Error(String),
}
pub(crate) fn start_worker(cancel: Arc<AtomicBool>) -> (Sender<Action>, Receiver<Reply>) {
    let (tx, commands) = mpsc::channel();
    let (replies, rx) = mpsc::channel();
    std::thread::spawn(move || worker(commands, replies, cancel));
    (tx, rx)
}
#[cfg(windows)]
fn worker(commands: Receiver<Action>, replies: Sender<Reply>, cancel: Arc<AtomicBool>) {
    use realme_buds::{Client, Error};
    use realme_buds_windows::{RfcommStream, paired_devices};
    let devices = paired_devices()
        .map(|d| d.into_iter().map(|d| (d.name, d.address)).collect())
        .unwrap_or_default();
    let _ = replies.send(Reply::Devices(devices));
    let mut client: Option<Client<RfcommStream>> = None;
    let mut active_address = String::new();
    let open = |address: &str| -> realme_buds::Result<Client<RfcommStream>> {
        let mut c = Client::new(RfcommStream::connect_service(
            address,
            realme_buds::CONTROL_SERVICE,
        )?);
        c.initialize()?;
        c.subscribe()?;
        Ok(c)
    };
    loop {
        let action = match commands.recv_timeout(Duration::from_millis(100)) {
            Ok(a) => a,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(c) = &mut client {
                    match c.next_event(Duration::from_millis(50)) {
                        Ok(e) => {
                            let _ = replies.send(Reply::Event(e));
                        }
                        Err(Error::Timeout) => {}
                        Err(e) => {
                            let _ = replies.send(Reply::Error(e.to_string()));
                            let _ = replies.send(Reply::Connected(false));
                            client = None;
                        }
                    }
                }
                continue;
            }
        };
        if matches!(action, Action::Disconnect) {
            client = None;
            let _ = replies.send(Reply::Connected(false));
            let _ = replies.send(Reply::Done("Disconnected".into()));
            continue;
        }
        if let Action::Connect(address) = action {
            client = None;
            active_address = address.clone();
            let result = open(&address);
            match result {
                Ok(mut c) => {
                    let _ = replies.send(Reply::Connected(true));
                    let _ = replies.send(Reply::State(Box::new(c.snapshot())));
                    client = Some(c);
                    let _ = replies.send(Reply::Done("Connected directly over Bluetooth".into()));
                }
                Err(e) => {
                    let _ = replies.send(Reply::Connected(false));
                    let _ = replies.send(Reply::Error(e.to_string()));
                }
            }
            continue;
        }
        let Some(c) = &mut client else {
            let _ = replies.send(Reply::Error("Connect first".into()));
            continue;
        };
        let disruptive = matches!(
            action,
            Action::Feature(Feature::DualDevice | Feature::HighDefinitionSound, _)
                | Action::Multipoint(_, _)
        );
        let intended_feature = match &action {
            Action::Feature(f, v) => Some((*f, *v)),
            _ => None,
        };
        let result = match action {
            Action::Refresh => Ok(()),
            Action::Audit => {
                let _ = c.set_timeout(Duration::from_secs(2));
                let _ = replies.send(Reply::Firmware(c.firmware_info()));
                let _ = replies.send(Reply::Diagnostics(c.property_audit()));
                let _ = c.set_timeout(Duration::from_secs(12));
                Ok(())
            }
            Action::Noise(m) => c.set_noise(m),
            Action::Anc(level) => c.set_anc_enabled(true, Some(level)),
            Action::Cycle(m) => c.set_noise_cycle(m),
            Action::Feature(f, v) => c.set_feature(f, v),
            Action::Volume(v) => c.set_volume_enhancer(v),
            Action::Preset(id) => c.set_eq_preset(id),
            Action::Eq(eq, a) => c.set_custom_eq(&eq, a),
            Action::Dynamic(b) => c.set_dynamic_band(b),
            Action::Bind(b) => c.set_touch(&[b]),
            Action::Multipoint(a, v) => c.multipoint_action(a, v),
            Action::Fit(true) => c
                .run_fit_test_with_cancel(Duration::from_secs(15), || {
                    cancel.load(Ordering::Relaxed)
                })
                .map(|v| {
                    let _ = replies.send(Reply::Event(Event::Fit(v)));
                }),
            Action::Fit(false) => c.fit_test(false),
            Action::Find(true) => c.find_buds_for_with_cancel(Duration::from_secs(3), || {
                cancel.load(Ordering::Relaxed)
            }),
            Action::Find(false) => c.find_buds(false),
            Action::Connect(_) | Action::Disconnect => unreachable!(),
        };
        if disruptive && (result.is_ok() || matches!(result, Err(Error::Io(_) | Error::Timeout))) {
            // Reconnect/read back only; never replay a write after a lost acknowledgement.
            client = None;
            std::thread::sleep(Duration::from_millis(750));
            match open(&active_address) {
                Ok(mut recovered) => {
                    let state = recovered.snapshot();
                    let matches = intended_feature.is_none_or(|(f, v)| {
                        state
                            .features
                            .iter()
                            .any(|s| s.id == f as u8 && s.enabled == v)
                    });
                    let _ = replies.send(Reply::State(Box::new(state)));
                    client = Some(recovered);
                    if matches {
                        let _ = replies.send(Reply::Done(
                            "Connection recovered; requested setting read back".into(),
                        ));
                    } else {
                        let _ = replies.send(Reply::Error("Connection recovered, but readback did not confirm the requested setting".into()));
                    }
                }
                Err(e) => {
                    let _ = replies.send(Reply::Connected(false));
                    let _ = replies.send(Reply::Error(format!(
                        "Setting may have changed. Reconnect manually: {e}"
                    )));
                }
            }
        } else {
            match result {
                Ok(()) | Err(Error::Cancelled) => {
                    let _ = replies.send(Reply::State(Box::new(c.snapshot())));
                    let _ = replies.send(Reply::Done("Stopped/refreshed; state read back".into()));
                }
                Err(e) => {
                    let _ = replies.send(Reply::Error(e.to_string()));
                }
            }
        }
    }
}
#[cfg(not(windows))]
fn worker(_commands: Receiver<Action>, replies: Sender<Reply>, _cancel: Arc<AtomicBool>) {
    let _ = replies.send(Reply::Error(
        "Built-in Bluetooth transport currently requires Windows".into(),
    ));
}
