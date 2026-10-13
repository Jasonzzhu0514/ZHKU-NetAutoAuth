#![windows_subsystem = "windows"]

mod app;
mod desktop;
mod paths;
mod portal;
mod portal_format;
mod storage;
mod theme;
mod updates;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    // Release verification must exit before loading credentials or starting GUI/network tasks.
    if std::env::args_os().skip(1).any(|arg| arg == "--version") {
        println!("ZHKU-NetAutoAuth {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--apply-update") {
        if let Some(manifest) = args.get(index + 1)
            && let Err(error) = updates::install::run_helper(std::path::Path::new(manifest))
        {
            updates::install::report_error(&error);
        }
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
