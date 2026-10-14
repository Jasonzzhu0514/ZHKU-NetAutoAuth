use serde::Serialize;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter, Manager};
use zhku_netautoauth_gui::{client::Client, updater::Service};

pub struct DesktopState {
    pub client: Mutex<Client>,
    pub updater: Mutex<Service>,
    pub network_tests: Mutex<zhku_netautoauth_gui::network_tests::Service>,
    pub running: AtomicBool,
    pub exiting: AtomicBool,
    pub tray_ready: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub client: zhku_netautoauth_gui::client::Snapshot,
    pub updater: zhku_netautoauth_gui::updater::Snapshot,
    pub tray_ready: bool,
}

impl DesktopState {
    pub fn new() -> Self {
        let mut client = Client::default();
        client.initialize(zhku_netautoauth_gui::updater::resume_requested());
        let mut updater = Service::default();
        updater.initialize();
        let mut network_tests = zhku_netautoauth_gui::network_tests::Service::default();
        network_tests.initialize();
        Self {
            client: Mutex::new(client),
            updater: Mutex::new(updater),
            network_tests: Mutex::new(network_tests),
            running: AtomicBool::new(true),
            exiting: AtomicBool::new(false),
            tray_ready: AtomicBool::new(false),
        }
    }

    pub fn view(&self) -> Result<View, String> {
        let client = self
            .client
            .lock()
            .map_err(|_| "客户端状态暂不可用")?
            .snapshot();
        let updater = self
            .updater
            .lock()
            .map_err(|_| "更新状态暂不可用")?
            .snapshot();
        Ok(View {
            client,
            updater,
            tray_ready: self.tray_ready.load(Ordering::Relaxed),
        })
    }
}

pub fn start_worker(app: AppHandle) {
    std::thread::spawn(move || {
        let mut tray_label = String::new();
        loop {
            let state = app.state::<DesktopState>();
            if !state.running.load(Ordering::Relaxed) {
                break;
            }
            let (online, check, changed, snapshot) = match state.client.lock() {
                Ok(mut client) => {
                    client.tick();
                    (
                        client.online(),
                        client.auto_check_updates(),
                        client.take_changed(),
                        client.snapshot(),
                    )
                }
                Err(_) => break,
            };
            if changed {
                let _ = app.emit("client-state", &snapshot);
                let network = match snapshot.network.state {
                    "online" => "已通过认证",
                    "offline" => "尚未认证",
                    _ => "状态待确认",
                };
                let label = format!(
                    "ZHKU-NetAutoAuth · {network} · {}",
                    if snapshot.guarding {
                        "守护运行中"
                    } else {
                        "守护已停止"
                    }
                );
                if label != tray_label {
                    crate::tray::update_status(&app, network, snapshot.guarding);
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_tooltip(Some(&label));
                    }
                    tray_label = label;
                }
            }
            if let Ok(mut updater) = state.updater.lock() {
                updater.automatic_check(check, online);
                if updater.poll() {
                    let _ = app.emit("updates-state", updater.snapshot());
                }
            }
            if let Ok(mut tests) = state.network_tests.lock()
                && tests.poll()
            {
                let _ = app.emit("network-test-state", tests.snapshot());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}
