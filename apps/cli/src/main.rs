// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
#[cfg_attr(not(windows), allow(dead_code))]
mod args;
#[cfg(windows)]
mod commands;
#[cfg(windows)]
use ::realme_buds::devices::air7 as realme_buds;
use args::{Args, Command};
use clap::Parser;
#[cfg(windows)]
use realme_buds::Client;
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    use realme_buds_windows::{RfcommStream, paired_devices};
    if matches!(args.command, Command::Devices) {
        commands::print_json(&paired_devices()?)?;
        return Ok(());
    }
    let address = match args.address {
        Some(a) => a,
        None => {
            let candidates: Vec<_> = paired_devices()?
                .into_iter()
                .filter(|d| d.name.to_ascii_lowercase().contains("buds air7"))
                .collect();
            if candidates.len() != 1 {
                return Err("use buds devices, then --address XX:XX:XX:XX:XX:XX".into());
            }
            candidates[0].address.clone()
        }
    };
    eprintln!("Connecting directly via Bluetooth Classic RFCOMM...");
    let stream = if let Some(channel) = args.channel {
        RfcommStream::connect_channel(&address, channel)?
    } else {
        RfcommStream::connect_service(&address, realme_buds::CONTROL_SERVICE)?
    };
    if let Some(path) = args.trace {
        let mut client = Client::new(::realme_buds::transport::trace::Traced::new(
            stream,
            std::fs::File::create(path)?,
        ));
        client.set_timeout(Duration::from_secs(args.timeout))?;
        return commands::execute(&mut client, args.command, args.raw);
    }
    let mut client = Client::new(stream);
    client.set_timeout(Duration::from_secs(args.timeout))?;
    commands::execute(&mut client, args.command, args.raw)
}
#[cfg(not(windows))]
fn run(_args: Args) -> Result<(), Box<dyn std::error::Error>> {
    Err("The built-in Bluetooth transport currently supports Windows. The library accepts a custom RFCOMM Transport on other platforms.".into())
}
fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
