use crate::state::DesktopState;
use tauri::{AppHandle, Emitter, State};
use zhku_netautoauth_gui::network_tests::Snapshot;

#[tauri::command]
pub fn network_tests_snapshot(state: State<'_, DesktopState>) -> Result<Snapshot, String> {
    Ok(state
        .network_tests
        .lock()
        .map_err(|_| "网站测试状态暂不可用")?
        .snapshot())
}

#[tauri::command]
pub fn network_tests_action(
    app: AppHandle,
    state: State<'_, DesktopState>,
    action: &str,
    id: Option<u64>,
    name: Option<String>,
    url: Option<String>,
) -> Result<Snapshot, String> {
    let mut tests = state
        .network_tests
        .lock()
        .map_err(|_| "网站测试状态暂不可用")?;
    match action {
        "test" => tests.start(id)?,
        "add" => tests.add(
            name.as_deref().ok_or("请输入网站名称")?,
            url.as_deref().ok_or("请输入网址")?,
        )?,
        "remove" => tests.remove(id.ok_or("请选择网站")?)?,
        _ => return Err("不支持的网站测试操作".into()),
    }
    let snapshot = tests.snapshot();
    let _ = app.emit("network-test-state", &snapshot);
    Ok(snapshot)
}
