#![windows_subsystem = "windows"]

mod app;
mod desktop;
mod paths;
mod portal;
mod portal_format;
mod storage;
mod theme;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    // Release verification must exit before loading credentials or starting GUI/network tasks.
    if std::env::args_os().skip(1).any(|arg| arg == "--version") {
        println!("ZHKU-NetAutoAuth {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    eframe::run_native(
        "ZHKU-NetAutoAuth",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(std::sync::Arc::new(theme::window_icon()))
                .with_inner_size([920.0, 760.0])
                .with_min_inner_size([480.0, 520.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
