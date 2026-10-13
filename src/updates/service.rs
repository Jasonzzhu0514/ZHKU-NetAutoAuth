mod backup;
mod download;
mod install;
mod platform;
mod registry;
mod releases;

use download::Prepared;
use releases::{Check, InstallMode, Release};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub version: String,
    pub notes: String,
    pub page: String,
    pub size: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub version: &'static str,
    pub edition: &'static str,
    pub message: String,
    pub failed: bool,
    pub rollback: bool,
    pub busy: bool,
    pub downloading: bool,
    pub bytes: u64,
    pub prepared: bool,
    pub release: Option<ReleaseInfo>,
}

enum Event {
    Checked(Result<Check, String>),
    Progress(u64),
    Downloaded(Result<Prepared, String>),
}

pub struct Service {
    target: Option<PathBuf>,
    mode: InstallMode,
    release: Option<Release>,
    prepared: Option<Prepared>,
    receiver: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    auto_checked: bool,
    message: String,
    failed: bool,
    rollback: bool,
    downloading: bool,
    bytes: u64,
    changed: bool,
    revision: u64,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            target: None,
            mode: InstallMode::Portable,
            release: None,
            prepared: None,
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            auto_checked: false,
            message: "尚未检查更新".into(),
            failed: false,
            rollback: false,
            downloading: false,
            bytes: 0,
            changed: true,
            revision: 0,
        }
    }
}

impl Service {
    pub fn initialize(&mut self) {
        self.target = std::env::current_exe()
            .ok()
            .and_then(|path| path.canonicalize().ok());
        if let Some(target) = &self.target {
            self.mode = platform::mode(target);
            install::cleanup_completed(target);
        }
        if install::failure_reported() {
            self.failed = true;
            self.rollback = true;
            self.message = "上次更新未完成，已恢复旧版。".into();
            if let Some(reason) = install::failure_reason() {
                self.message.push_str(&format!("\n原因：{reason}"));
            }
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            version: env!("CARGO_PKG_VERSION"),
            edition: if self.mode == InstallMode::Installed {
                "安装版"
            } else {
                "便携版"
            },
            message: self.message.clone(),
            failed: self.failed,
            rollback: self.rollback,
            busy: self.receiver.is_some(),
            downloading: self.downloading,
            bytes: self.bytes,
            prepared: self.prepared.is_some(),
            release: self.release.as_ref().map(|release| ReleaseInfo {
                version: release.version.to_string(),
                notes: release.notes.clone(),
                page: release.page.clone(),
                size: release.size,
            }),
        }
    }

    pub fn check(&mut self) -> Result<(), String> {
        if self.receiver.is_some() {
            return Err("请等待当前更新任务完成".into());
        }
        self.release = None;
        self.prepared = None;
        self.failed = false;
        self.rollback = false;
        self.message = "正在检查更新…".into();
        self.mark_changed();
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let mode = self.mode;
        std::thread::spawn(move || {
            let _ = sender.send(Event::Checked(releases::latest(mode)));
        });
        Ok(())
    }

    pub fn automatic_check(&mut self, enabled: bool, online: bool) {
        if !enabled || !online || self.auto_checked {
            return;
        }
        if self.release.is_some() {
            self.auto_checked = true;
        } else if self.receiver.is_none() {
            self.auto_checked = true;
            let _ = self.check();
        }
    }

    pub fn download(&mut self) -> Result<(), String> {
        if self.receiver.is_some() {
            return Err("请等待当前更新任务完成".into());
        }
        let release = self.release.clone().ok_or("请先检查更新")?;
        let target = self.target.clone().ok_or("无法确定当前程序位置")?;
        self.failed = false;
        self.downloading = true;
        self.bytes = 0;
        self.message = "正在下载更新…".into();
        self.mark_changed();
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let result = download::download(&release, &target, &cancel, |bytes| {
                let _ = sender.send(Event::Progress(bytes));
            });
            let _ = sender.send(Event::Downloaded(result));
        });
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.message = "正在取消下载…".into();
        self.mark_changed();
    }

    pub fn install(&mut self, resume_guard: bool) -> Result<(), String> {
        let prepared = self.prepared.take().ok_or("请先下载并校验更新包")?;
        let release = self.release.as_ref().ok_or("更新信息已失效，请重新检查")?;
        if let Err(error) = install::launch(prepared, release, self.mode, resume_guard) {
            self.message = error.clone();
            self.failed = true;
            self.mark_changed();
            return Err(error);
        }
        Ok(())
    }

    pub fn poll(&mut self) -> bool {
        let mut complete = false;
        let mut progress = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(Event::Progress(bytes)) => {
                        self.bytes = bytes;
                        progress = true;
                    }
                    Ok(Event::Checked(result)) => {
                        complete = true;
                        match result {
                            Ok(Check::Current(version)) => {
                                self.message = format!("未发现更新（最新正式版 {version}）")
                            }
                            Ok(Check::Update(release)) => {
                                self.message = format!("发现新版本 {}", release.version);
                                self.release = Some(release);
                            }
                            Err(error) => {
                                self.message = error;
                                self.failed = true;
                            }
                        }
                    }
                    Ok(Event::Downloaded(result)) => {
                        complete = true;
                        self.downloading = false;
                        match result {
                            Ok(prepared) => {
                                self.prepared = Some(prepared);
                                self.message = "下载完成，校验通过".into();
                            }
                            Err(error) => {
                                self.message = error;
                                self.failed = true;
                            }
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !complete {
                            self.message = "更新任务异常结束，请重试".into();
                            self.failed = true;
                            self.downloading = false;
                        }
                        complete = true;
                        break;
                    }
                }
            }
        }
        if progress {
            self.mark_changed();
        }
        if complete {
            self.receiver = None;
            self.mark_changed();
        }
        std::mem::take(&mut self.changed)
    }

    pub fn release_page(&self) -> &str {
        self.release
            .as_ref()
            .map_or(releases::RELEASES_URL, |release| release.page.as_str())
    }

    fn mark_changed(&mut self) {
        self.revision += 1;
        self.changed = true;
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn helper_entry() -> bool {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--apply-update") {
        if let Some(manifest) = args.get(index + 1)
            && let Err(error) = install::run_helper(std::path::Path::new(manifest))
        {
            install::report_error(&error);
        }
        return true;
    }
    false
}

pub fn resume_requested() -> bool {
    install::resume_requested()
}
pub fn confirm_startup() {
    install::confirm_startup();
}
