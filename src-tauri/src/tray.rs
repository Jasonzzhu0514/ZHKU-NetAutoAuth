use crate::state::DesktopState;
use std::sync::atomic::Ordering;
use tauri::{
    App, AppHandle, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub struct TrayStatus {
    network: MenuItem<tauri::Wry>,
    guard: MenuItem<tauri::Wry>,
}

pub fn update_status(app: &AppHandle, network: &str, guarding: bool) {
    if let Some(status) = app.try_state::<TrayStatus>() {
        let _ = status.network.set_text(format!("网络：{network}"));
        let _ = status.guard.set_text(if guarding {
            "自动守护：运行中"
        } else {
            "自动守护：已停止"
        });
    }
}

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn install(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "show", "打开窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let network = MenuItem::with_id(app, "network", "网络：状态待确认", false, None::<&str>)?;
    let guard = MenuItem::with_id(app, "guard", "自动守护：已停止", false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&network, &guard, &separator, &open, &quit])?;
    let mut rgba = vec![0; 32 * 32 * 4];
    for y in 0..32usize {
        for x in 0..32usize {
            let node = [(16_i32, 8_i32), (8, 24), (24, 24)]
                .iter()
                .any(|(cx, cy)| (x as i32 - cx).pow(2) + (y as i32 - cy).pow(2) <= 16);
            let line = ((15..=16).contains(&x) && (8..=20).contains(&y))
                || ((8..=24).contains(&x) && (19..=20).contains(&y))
                || ([8, 24].contains(&x) && (20..=24).contains(&y));
            if node || line {
                rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[0, 120, 212, 255]);
            }
        }
    }
    let icon = tauri::image::Image::new_owned(rgba, 32, 32);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_icon(icon.clone());
    }
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("ZHKU-NetAutoAuth")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show(app),
            "quit" => {
                app.state::<DesktopState>()
                    .exiting
                    .store(true, Ordering::Relaxed);
                app.state::<DesktopState>()
                    .running
                    .store(false, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } | TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(TrayStatus { network, guard });
    Ok(())
}
