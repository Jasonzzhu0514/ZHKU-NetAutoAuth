mod accounts;
mod model;
mod network;
#[cfg(test)]
mod tests;

use crate::{portal, preferences, storage};
pub use model::{AccountInput, DesktopPreferences, LogEntry, Snapshot};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub enum Action {
    Check,
    Login,
    Guard,
    Logout,
}

struct NetworkEvent {
    status: portal::Status,
    message: String,
    failed: bool,
}

pub struct Client {
    settings: storage::Settings,
    saved_username: Option<String>,
    preferences: preferences::Preferences,
    status: portal::Status,
    guard: bool,
    guard_pending: bool,
    failures: u32,
    next_check: Instant,
    receiver: Option<mpsc::Receiver<NetworkEvent>>,
    trace_receiver: Option<mpsc::Receiver<portal::Trace>>,
    logs: Vec<LogEntry>,
    next_log_id: u64,
    log_generation: u64,
    started_at: Instant,
    changed: bool,
    revision: u64,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            settings: storage::Settings::default(),
            saved_username: None,
            preferences: preferences::Preferences::default(),
            status: portal::Status::unknown("检测中", "正在查询网络状态"),
            guard: false,
            guard_pending: false,
            failures: 0,
            next_check: Instant::now(),
            receiver: None,
            trace_receiver: None,
            logs: Vec::new(),
            next_log_id: 1,
            log_generation: 0,
            started_at: Instant::now(),
            changed: true,
            revision: 0,
        }
    }
}

impl Client {
    pub fn initialize(&mut self, resume_guard: bool) {
        match storage::path().and_then(|path| storage::load(&path)) {
            Ok(settings) => {
                self.saved_username =
                    (!settings.username.is_empty()).then(|| settings.username.clone());
                self.settings = settings;
            }
            Err(error) => self.log(error, true),
        }
        match crate::paths::app_data()
            .map(|path| path.join("desktop.json"))
            .and_then(|path| preferences::Preferences::load(&path))
        {
            Ok(preferences) => self.preferences = preferences,
            Err(error) => self.log(error, true),
        }
        if self.settings.auto_login || (resume_guard && self.configured()) {
            self.guard = true;
            self.log("已开启启动自动登录和守护", false);
            let _ = self.launch(Action::Guard);
        } else {
            let _ = self.launch(Action::Check);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            network: (&self.status).into(),
            busy: self.receiver.is_some(),
            guarding: self.guard,
            username: self.settings.username.clone(),
            saved_username: self.saved_username.clone(),
            remember_password: self.settings.remember,
            auto_login: self.settings.auto_login,
            has_password: !self.settings.password.is_empty(),
            log_generation: self.log_generation,
            log_count: self.logs.len(),
            preferences: (&self.preferences).into(),
        }
    }

    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    fn mark_changed(&mut self) {
        self.revision += 1;
        self.changed = true;
    }

    pub fn online(&self) -> bool {
        self.status.state == portal::State::Online
    }

    pub fn resume_guard_after_update(&self) -> bool {
        self.guard && self.settings.remember
    }

    pub fn auto_check_updates(&self) -> bool {
        self.preferences.auto_check_updates
    }

    pub fn should_hide_on_close(&self) -> bool {
        self.preferences.close == preferences::CloseBehavior::Tray
            && self.preferences.notice_acknowledged
    }

    pub fn notice_pending(&self) -> bool {
        !self.preferences.notice_acknowledged
    }

    pub fn set_preferences(&mut self, input: DesktopPreferences) -> Result<(), String> {
        let next = preferences::Preferences {
            close: match input.close_behavior.as_str() {
                "tray" => preferences::CloseBehavior::Tray,
                "exit" => preferences::CloseBehavior::Exit,
                _ => return Err("无效的关闭窗口选项".into()),
            },
            notice_acknowledged: input.notice_acknowledged,
            auto_check_updates: input.auto_check_updates,
        };
        let path = crate::paths::app_data()?.join("desktop.json");
        next.save(&path)?;
        self.preferences = next;
        self.mark_changed();
        Ok(())
    }

    pub fn logs_after(&self, after: u64) -> Vec<LogEntry> {
        let start = self.logs.partition_point(|log| log.id <= after);
        self.logs[start..].iter().take(256).cloned().collect()
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.log_generation += 1;
        self.mark_changed();
    }

    fn log(&mut self, message: impl Into<String>, failed: bool) {
        self.logs.push(LogEntry {
            id: self.next_log_id,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            level: if failed { "ERROR" } else { "INFO" },
            message: message.into(),
        });
        self.next_log_id += 1;
        self.mark_changed();
    }

    fn configured(&self) -> bool {
        !self.settings.username.trim().is_empty() && !self.settings.password.is_empty()
    }
}
