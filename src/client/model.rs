use crate::{portal, preferences};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub state: &'static str,
    pub region: String,
    pub ip: String,
    pub account: String,
    pub mac: String,
    pub cvid: String,
    pub pvid: String,
    pub detail: String,
}

impl From<&portal::Status> for NetworkStatus {
    fn from(value: &portal::Status) -> Self {
        Self {
            state: match value.state {
                portal::State::Online => "online",
                portal::State::Offline => "offline",
                portal::State::Unknown => "unknown",
            },
            region: value.region.into(),
            ip: value.ip.clone(),
            account: value.account.clone(),
            mac: value.online_mac.clone(),
            cvid: value.cvid.clone(),
            pvid: value.pvid.clone(),
            detail: value.detail.clone(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub network: NetworkStatus,
    pub busy: bool,
    pub guarding: bool,
    pub username: String,
    pub saved_username: Option<String>,
    pub remember_password: bool,
    pub auto_login: bool,
    pub has_password: bool,
    pub log_generation: u64,
    pub log_count: usize,
    pub preferences: DesktopPreferences,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreferences {
    pub close_behavior: String,
    pub notice_acknowledged: bool,
    pub auto_check_updates: bool,
    pub guard_interval_seconds: u64,
}

impl From<&preferences::Preferences> for DesktopPreferences {
    fn from(value: &preferences::Preferences) -> Self {
        Self {
            close_behavior: if value.close == preferences::CloseBehavior::Tray {
                "tray"
            } else {
                "exit"
            }
            .into(),
            notice_acknowledged: value.notice_acknowledged,
            auto_check_updates: value.auto_check_updates,
            guard_interval_seconds: value.guard_interval_seconds,
        }
    }
}

// Credentials cross only the local desktop IPC, never state events or log entries.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub username: String,
    pub password: String,
    pub remember_password: bool,
    pub auto_login: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: u64,
    pub elapsed_ms: u64,
    pub level: &'static str,
    pub message: String,
}
