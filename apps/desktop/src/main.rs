// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod worker;

fn main() -> eframe::Result {
    eframe::run_native(
        "OpenBuds Air7",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1040.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(app::App::new()))),
    )
}
