use crate::state::{DesktopState, View};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use zhku_netautoauth_gui::client::{AccountInput, Action, DesktopPreferences, LogEntry};

#[tauri::command]
pub fn snapshot(state: State<'_, DesktopState>) -> Result<View, String> {
    state.view()
}

#[tauri::command]
pub fn credentials(state: State<'_, DesktopState>) -> Result<AccountInput, String> {
    Ok(state
        .client
        .lock()
        .map_err(|_| "账号状态暂不可用")?
        .account())
}

#[tauri::command]
pub fn save_account(
    account: AccountInput,
    state: State<'_, DesktopState>,
) -> Result<Option<String>, String> {
    state
        .client
        .lock()
        .map_err(|_| "账号状态暂不可用")?
        .set_account(account)
}

#[tauri::command]
pub fn persist_account(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .client
        .lock()
        .map_err(|_| "账号状态暂不可用")?
        .persist_account()
}

#[tauri::command]
pub fn restore_account(state: State<'_, DesktopState>) -> Result<AccountInput, String> {
    state
        .client
        .lock()
        .map_err(|_| "账号状态暂不可用")?
        .restore_account()
}

#[tauri::command]
pub fn delete_account(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .client
        .lock()
        .map_err(|_| "账号状态暂不可用")?
        .forget_account()
}

#[tauri::command]
pub fn network_action(action: &str, state: State<'_, DesktopState>) -> Result<(), String> {
    let action = match action {
        "check" => Action::Check,
        "login" => Action::Login,
        "logout" => Action::Logout,
        _ => return Err("不支持的网络操作".into()),
    };
    state
        .client
        .lock()
        .map_err(|_| "网络状态暂不可用")?
        .launch(action)
}

#[tauri::command]
pub fn set_guard(enabled: bool, state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .client
        .lock()
        .map_err(|_| "守护状态暂不可用")?
        .set_guard(enabled)
}

#[tauri::command]
pub fn save_preferences(
    preferences: DesktopPreferences,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    state
        .client
        .lock()
        .map_err(|_| "设置暂不可用")?
        .set_preferences(preferences)
}

#[tauri::command]
pub fn get_logs(after: u64, state: State<'_, DesktopState>) -> Result<Vec<LogEntry>, String> {
    Ok(state
        .client
        .lock()
        .map_err(|_| "日志暂不可用")?
        .logs_after(after))
}

#[tauri::command]
pub fn clear_logs(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .client
        .lock()
        .map_err(|_| "日志暂不可用")?
        .clear_logs();
    Ok(())
}

#[tauri::command]
pub fn check_updates(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "更新状态暂不可用")?
        .check()
}

#[tauri::command]
pub fn download_update(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "更新状态暂不可用")?
        .download()
}

#[tauri::command]
pub fn cancel_update(state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .updater
        .lock()
        .map_err(|_| "更新状态暂不可用")?
        .cancel();
    Ok(())
}

#[tauri::command]
pub fn install_update(app: AppHandle, state: State<'_, DesktopState>) -> Result<(), String> {
    let resume = state
        .client
        .lock()
        .map_err(|_| "守护状态暂不可用")?
        .resume_guard_after_update();
    state
        .updater
        .lock()
        .map_err(|_| "更新状态暂不可用")?
        .install(resume)?;
    state.exiting.store(true, Ordering::Relaxed);
    state.running.store(false, Ordering::Relaxed);
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn open_release_page(app: AppHandle, state: State<'_, DesktopState>) -> Result<(), String> {
    let updater = state.updater.lock().map_err(|_| "更新状态暂不可用")?;
    app.opener()
        .open_url(updater.release_page(), None::<&str>)
        .map_err(|_| "无法打开发布页面".into())
}

#[tauri::command]
pub fn open_authentication_page(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    let url = state
        .client
        .lock()
        .map_err(|_| "网络状态暂不可用")?
        .authentication_page()
        .ok_or("请先刷新状态，确定所在区域")?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| "无法打开认证页面，请检查默认浏览器".into())
}

#[tauri::command]
pub fn frontend_ready() {
    zhku_netautoauth_gui::updater::confirm_startup();
}

#[tauri::command]
pub fn close_window(app: AppHandle, state: State<'_, DesktopState>) -> Result<(), String> {
    let client = state.client.lock().map_err(|_| "客户端状态暂不可用")?;
    if client.notice_pending() && state.tray_ready.load(Ordering::Relaxed) {
        let _ = app.emit("show-notice", ());
        return Ok(());
    }
    if client.should_hide_on_close() && state.tray_ready.load(Ordering::Relaxed) {
        let window = app.get_webview_window("main").ok_or("主窗口暂不可用")?;
        let _ = app.emit("window-hidden", ());
        window.hide().map_err(|_| "无法隐藏窗口".into())
    } else {
        state.exiting.store(true, Ordering::Relaxed);
        state.running.store(false, Ordering::Relaxed);
        app.exit(0);
        Ok(())
    }
}
