// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod demo;
mod worker;

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "OpenBuds Air7\nUsage: buds-desktop [--demo]\n--demo: offline synthetic preview; no Bluetooth I/O"
        );
        return Ok(());
    }
    if args.iter().any(|a| a != "--demo") {
        eprintln!("Unknown option. Use --help.");
        std::process::exit(2);
    }
    let demo = args.iter().any(|a| a == "--demo");
    eframe::run_native(
        "OpenBuds Air7",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1040.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(app::App::new(demo)))),
    )
}
