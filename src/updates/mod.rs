mod backup;
mod download;
pub(crate) mod install;
mod platform;
mod registry;
mod releases;
mod ui;

use download::Prepared;
use eframe::egui;
use releases::{Check, InstallMode, Release};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

enum Event {
    Checked(Result<Check, String>),
    Progress(u64),
    Downloaded(Result<Prepared, String>),
}

pub(crate) struct Updater {
    mode: InstallMode,
    target: Option<PathBuf>,
    release: Option<Release>,
    prepared: Option<Prepared>,
    receiver: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    checked_automatically: bool,
    message: String,
    failed: bool,
    downloading: bool,
    bytes: u64,
    confirm_install: bool,
    install_requested: bool,
    automatic_request: bool,
    notify: bool,
    settings_requested: bool,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            mode: InstallMode::Portable,
            target: None,
            release: None,
            prepared: None,
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            checked_automatically: false,
            message: "尚未检查更新".into(),
            failed: false,
            downloading: false,
            bytes: 0,
            confirm_install: false,
            install_requested: false,
            automatic_request: false,
            notify: false,
            settings_requested: false,
        }
    }
}

impl Updater {
    pub fn initialize(&mut self) {
        self.target = std::env::current_exe()
            .ok()
            .and_then(|path| path.canonicalize().ok());
        if let Some(target) = &self.target {
            self.mode = platform::mode(target);
            install::cleanup_completed(target);
        }
        if install::failure_reported() {
            self.message = "上次更新未完成，已恢复旧版。可以重新检查或手动下载安装。".into();
            if let Some(reason) = install::failure_reason() {
                self.message.push_str(&format!("\n原因：{reason}"));
            }
            self.failed = true;
        }
    }

    pub fn automatic_check(&mut self, ctx: &egui::Context, enabled: bool, online: bool) {
        if enabled && online && !self.checked_automatically && self.receiver.is_none() {
            self.checked_automatically = true;
            self.check(ctx, true);
        }
    }

    fn check(&mut self, ctx: &egui::Context, automatic: bool) {
        if self.receiver.is_some() {
            return;
        }
        self.release = None;
        self.prepared = None;
        self.failed = false;
        self.automatic_request = automatic;
        self.message = "正在检查更新…".into();
        let (send, receive) = mpsc::channel();
        self.receiver = Some(receive);
        let mode = self.mode;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(Event::Checked(releases::latest(mode)));
            ctx.request_repaint();
        });
    }

    fn download(&mut self, ctx: &egui::Context) {
        let (Some(release), Some(target)) = (self.release.clone(), self.target.clone()) else {
            return;
        };
        if self.receiver.is_some() {
            return;
        }
        self.failed = false;
        self.downloading = true;
        self.bytes = 0;
        self.message = "正在下载更新…".into();
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let (send, receive) = mpsc::channel();
        self.receiver = Some(receive);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = download::download(&release, &target, &cancel, |bytes| {
                let _ = send.send(Event::Progress(bytes));
            });
            let _ = send.send(Event::Downloaded(result));
            ctx.request_repaint();
        });
    }

    pub fn poll(&mut self) {
        let mut complete = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(Event::Progress(bytes)) => self.bytes = bytes,
                    Ok(Event::Checked(result)) => {
                        complete = true;
                        match result {
                            Ok(Check::Update(release)) => {
                                self.message = format!("发现新版本 {}", release.version);
                                self.release = Some(release);
                                self.notify = self.automatic_request;
                            }
                            Ok(Check::Current(version)) => {
                                self.message = format!("当前已是最新版本（发布版本 {version}）")
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
        if complete {
            self.receiver = None;
        }
    }

    pub fn install_requested(&mut self) -> bool {
        std::mem::take(&mut self.install_requested)
    }
    pub fn settings_requested(&mut self) -> bool {
        std::mem::take(&mut self.settings_requested)
    }
    pub fn has_update(&self) -> bool {
        self.release.is_some()
    }

    pub fn installation_error(&mut self, message: &str) {
        self.message = message.into();
        self.failed = true;
    }

    pub fn install(&mut self, resume_guard: bool) -> bool {
        let Some(prepared) = self.prepared.take() else {
            return false;
        };
        let Some(release) = &self.release else {
            return false;
        };
        match install::launch(prepared, release, self.mode, resume_guard) {
            Ok(()) => true,
            Err(error) => {
                self.message = error;
                self.failed = true;
                false
            }
        }
    }
}

impl Drop for Updater {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
