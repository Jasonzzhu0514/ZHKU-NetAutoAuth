#![windows_subsystem = "windows"]

mod commands;
mod state;
mod tray;

use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};

fn main() {
    if std::env::args_os().any(|arg| arg == "--version") {
        println!("ZHKU-NetAutoAuth {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if zhku_netautoauth_gui::updater::helper_entry() {
        return;
    }
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(state::DesktopState::new());
            let ready = tray::install(app).is_ok();
            app.state::<state::DesktopState>()
                .tray_ready
                .store(ready, Ordering::Relaxed);
            state::start_worker(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<state::DesktopState>();
                if !state.exiting.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.app_handle().emit("request-close", ());
                }
            }
            if matches!(event, tauri::WindowEvent::Focused(false)) {
                let _ = window.app_handle().emit("window-hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::credentials,
            commands::save_account,
            commands::persist_account,
            commands::restore_account,
            commands::delete_account,
            commands::network_action,
            commands::set_guard,
            commands::save_preferences,
            commands::get_logs,
            commands::clear_logs,
            commands::check_updates,
            commands::download_update,
            commands::cancel_update,
            commands::install_update,
            commands::open_release_page,
            commands::frontend_ready,
            commands::close_window,
        ])
        .build(tauri::generate_context!())
        .expect("无法启动 ZHKU-NetAutoAuth 桌面窗口");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<state::DesktopState>()
                .running
                .store(false, Ordering::Relaxed);
        }
    });
}
