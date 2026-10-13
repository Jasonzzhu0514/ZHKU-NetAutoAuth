use crate::{portal, storage, theme};
#[cfg(test)]
#[path = "tests/layout.rs"]
mod layout_tests;
#[path = "ui/mod.rs"]
mod ui;
use crate::portal::{Credentials, Portal, State, Status};
use eframe::egui;
use std::{
    collections::VecDeque,
    sync::mpsc,
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

enum Action {
    Check,
    Login,
    Guard,
    Logout,
}

struct LogEntry {
    elapsed: Duration,
    message: String,
    failed: bool,
}
struct Event {
    status: Status,
    message: String,
    failed: bool,
}

pub(crate) struct App {
    desktop: crate::desktop::Desktop,
    updater: crate::updates::Updater,
    startup_confirmed: bool,
    username: String,
    saved_username: Option<String>,
    password: Zeroizing<String>,
    show_password: bool,
    remember_password: bool,
    auto_login: bool,
    settings_dirty: bool,
    guard: bool,
    guard_pending: bool,
    status: Status,
    receiver: Option<mpsc::Receiver<Event>>,
    trace_receiver: Option<mpsc::Receiver<portal::Trace>>,
    next_check: Instant,
    failures: u32,
    logs: VecDeque<LogEntry>,
    log_view: ui::log_view::LogView,
    started_at: Instant,
    errors_only: bool,
    show_logs: bool,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_context(&cc.egui_ctx);
        let mut app = Self::empty();
        app.updater.initialize();
        for error in app.desktop.initialize(cc) {
            app.log_entry(error, true);
        }
        if crate::updates::install::failure_reported() {
            app.desktop.open_settings();
        }
        match storage::path().and_then(|file| storage::load(&file)) {
            Ok(settings) => {
                if !settings.username.is_empty() {
                    app.saved_username = Some(settings.username.clone());
                }
                app.username = settings.username;
                app.password = settings.password;
                app.remember_password = settings.remember;
                app.auto_login = settings.auto_login;
            }
            Err(error) => app.log_entry(error, true),
        }
        if app.auto_login
            || (crate::updates::install::resume_requested()
                && !app.username.trim().is_empty()
                && !app.password.is_empty())
        {
            app.guard = true;
            app.log(if app.auto_login {
                "已开启启动自动登录和守护"
            } else {
                "更新后已恢复自动守护"
            });
            app.launch(Action::Guard, &cc.egui_ctx);
        } else {
            app.launch(Action::Check, &cc.egui_ctx);
        }
        app
    }

    fn configure_context(ctx: &egui::Context) {
        theme::configure(ctx);
    }

    fn empty() -> Self {
        Self {
            desktop: crate::desktop::Desktop::default(),
            updater: crate::updates::Updater::default(),
            startup_confirmed: false,
            username: String::new(),
            saved_username: None,
            password: Zeroizing::new(String::new()),
            show_password: false,
            remember_password: false,
            auto_login: false,
            settings_dirty: false,
            guard: false,
            guard_pending: false,
            status: Status::unknown("检测中", "正在查询网络状态"),
            receiver: None,
            trace_receiver: None,
            next_check: Instant::now(),
            failures: 0,
            logs: VecDeque::new(),
            log_view: ui::log_view::LogView::default(),
            started_at: Instant::now(),
            errors_only: false,
            show_logs: false,
        }
    }

    fn persist_settings(&mut self) {
        if !self.remember_password || self.username.trim().is_empty() || self.password.is_empty() {
            self.auto_login = false;
        }
        let settings = storage::Settings {
            username: self.username.clone(),
            password: if self.remember_password {
                self.password.clone()
            } else {
                Zeroizing::new(String::new())
            },
            remember: self.remember_password,
            auto_login: self.auto_login,
        };
        match storage::path().and_then(|file| storage::save(&file, &settings)) {
            Ok(()) => {
                self.settings_dirty = false;
                self.saved_username = if self.username.trim().is_empty() {
                    None
                } else {
                    Some(self.username.trim().to_owned())
                };
            }
            Err(error) => self.log_entry(error, true),
        }
    }

    fn forget_credentials(&mut self, ctx: &egui::Context) {
        if let Err(error) = storage::path().and_then(|file| storage::delete(&file)) {
            self.log_entry(error, true);
            return;
        }
        self.username.zeroize();
        self.password.zeroize();
        self.show_password = false;
        self.remember_password = false;
        self.auto_login = false;
        self.settings_dirty = false;
        self.saved_username = None;
        self.guard = false;
        self.guard_pending = false;
        ctx.data_mut(|data| {
            data.remove::<egui::text_edit::TextEditState>(egui::Id::new("campus-account"));
            data.remove::<egui::text_edit::TextEditState>(egui::Id::new("campus-password"));
        });
        self.log("已删除 Rust 版保存的凭据，并清除本次输入");
    }

    fn log(&mut self, message: impl Into<String>) {
        self.log_entry(message, false);
    }

    fn log_entry(&mut self, message: impl Into<String>, failed: bool) {
        self.logs.push_back(LogEntry {
            elapsed: self.started_at.elapsed(),
            message: message.into(),
            failed,
        });
    }

    fn launch(&mut self, action: Action, ctx: &egui::Context) {
        if self.receiver.is_some() {
            return;
        }
        if matches!(action, Action::Check) {
            self.log("正在刷新网络状态…");
        }
        if matches!(action, Action::Logout) {
            self.guard = false;
            self.guard_pending = false;
            if self.auto_login {
                self.auto_login = false;
                self.persist_settings();
            }
            self.log("正在注销，自动守护已停止");
        }
        if matches!(action, Action::Login) {
            self.persist_settings();
        }
        let credentials = if matches!(action, Action::Check | Action::Logout) {
            None
        } else {
            Some(Credentials {
                username: self.username.trim().into(),
                password: self.password.clone(),
            })
        };
        let (tx, rx) = mpsc::channel();
        let (trace_tx, trace_rx) = mpsc::channel();
        self.trace_receiver = Some(trace_rx);
        self.receiver = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let event = match Portal::new().map(|p| p.with_trace(trace_tx)) {
                Err(error) => Event {
                    status: Status::unknown("未检测到", &error),
                    message: error,
                    failed: true,
                },
                Ok(portal) if matches!(action, Action::Logout) => match portal.logout() {
                    Ok(status) => Event {
                        status,
                        message: "已确认离线，自动守护保持停止".into(),
                        failed: false,
                    },
                    Err(error) => Event {
                        status: portal.status(),
                        message: error,
                        failed: true,
                    },
                },
                Ok(portal) => {
                    let status = portal.status();
                    let should_login =
                        !matches!(action, Action::Check) && status.state == State::Offline;
                    if should_login {
                        match portal.login(credentials.as_ref().expect("login credentials")) {
                            Ok(status) => Event {
                                status,
                                message: "认证成功，已确认在线".into(),
                                failed: false,
                            },
                            Err(error) => Event {
                                status: portal.status(),
                                message: error,
                                failed: true,
                            },
                        }
                    } else {
                        let failed = status.state == State::Unknown;
                        let message = match status.state {
                            State::Online => "已通过认证".into(),
                            State::Offline => "已连接校园网，尚未认证".into(),
                            State::Unknown => status.detail.clone(),
                        };
                        Event {
                            status,
                            message,
                            failed,
                        }
                    }
                }
            };
            let _ = tx.send(event);
            ctx.request_repaint();
        });
    }

    fn restore_saved_account(&mut self) {
        match storage::path().and_then(|file| storage::load(&file)) {
            Ok(settings) => {
                self.username = settings.username;
                self.password = settings.password;
                self.remember_password = settings.remember;
                self.auto_login = settings.auto_login;
                self.settings_dirty = false;
                self.show_password = false;
            }
            Err(error) => self.log_entry(error, true),
        }
    }

    fn drain_traces(&mut self) {
        if let Some(receiver) = &self.trace_receiver {
            for trace in receiver.try_iter() {
                self.logs.push_back(LogEntry {
                    elapsed: trace.at.saturating_duration_since(self.started_at),
                    message: trace.message,
                    failed: trace.failed,
                });
            }
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.desktop.shutdown();
        if self.settings_dirty {
            self.persist_settings();
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.updater.poll();
        if self.updater.settings_requested() {
            self.desktop.open_settings();
        }
        if self.updater.install_requested() {
            if self.settings_dirty {
                self.persist_settings();
            }
            if self.settings_dirty {
                self.updater
                    .installation_error("账号设置未能保存，已暂停安装。请查看运行日志后重试。");
            } else if self.updater.install(self.guard && self.remember_password) {
                self.desktop.exit(ctx);
                self.guard = false;
                self.guard_pending = false;
                return;
            }
        }
        self.updater.automatic_check(
            ctx,
            self.desktop.auto_check_updates(),
            self.status.state == State::Online,
        );
        if self.desktop.update(ctx, &self.status, self.guard) {
            self.guard = false;
            self.guard_pending = false;
            return;
        }
        self.drain_traces();
        let received = self.receiver.as_ref().map(|r| r.try_recv());
        match received {
            Some(Ok(event)) => {
                self.drain_traces();
                self.receiver = None;
                self.status = event.status;
                self.failures = if event.failed {
                    (self.failures + 1).min(5)
                } else {
                    0
                };
                let delay = (30 * 2u64.pow(self.failures)).min(300);
                self.next_check = Instant::now()
                    + if self.guard_pending {
                        Duration::ZERO
                    } else {
                        Duration::from_secs(delay)
                    };
                self.guard_pending = false;
                self.log_entry(event.message, event.failed);
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.receiver = None;
                self.guard = false;
                self.guard_pending = false;
                self.log("网络任务异常结束，自动守护已停止");
            }
            _ => {}
        }
        if self.guard && self.receiver.is_none() && Instant::now() >= self.next_check {
            self.launch(Action::Guard, ctx);
        }
        ctx.request_repaint_after(Duration::from_secs(1));
        if self.desktop.is_hidden() {
            self.show_password = false;
        } else {
            self.draw(ctx);
            self.updater.notice(ctx);
            self.desktop.ui(ctx, &mut self.updater);
        }
        if !self.startup_confirmed {
            crate::updates::install::confirm_startup();
            self.startup_confirmed = true;
        }
    }
}
