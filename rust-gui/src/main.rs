#![windows_subsystem = "windows"]

mod portal;
mod storage;
use eframe::egui;
use portal::{Credentials, Portal, State, Status};
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

struct App {
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
    started_at: Instant,
    errors_only: bool,
    show_logs: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_context(&cc.egui_ctx);
        let mut app = Self::empty();
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
        if app.auto_login {
            app.guard = true;
            app.log("已开启启动自动登录和守护");
            app.launch(Action::Guard, &cc.egui_ctx);
        } else {
            app.launch(Action::Check, &cc.egui_ctx);
        }
        app
    }

    fn configure_context(ctx: &egui::Context) {
        // Load Chinese glyphs from the system; never embed a user's local path.
        if let Some(windows) = std::env::var_os("WINDIR") {
            let fonts_dir = std::path::PathBuf::from(windows).join("Fonts");
            for file in ["msyh.ttc", "simhei.ttf", "simsun.ttc"] {
                if let Ok(bytes) = std::fs::read(fonts_dir.join(file)) {
                    let mut fonts = egui::FontDefinitions::default();
                    fonts
                        .font_data
                        .insert("chinese".into(), egui::FontData::from_owned(bytes).into());
                    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                        fonts
                            .families
                            .entry(family)
                            .or_default()
                            .insert(0, "chinese".into());
                    }
                    ctx.set_fonts(fonts);
                    break;
                }
            }
        }
        ctx.set_visuals(egui::Visuals::light());
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
        ctx.set_style(style);
    }

    fn empty() -> Self {
        Self {
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
            started_at: Instant::now(),
            errors_only: false,
            show_logs: false,
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        let background = egui::Color32::from_rgb(242, 245, 250);
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new().fill(background).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("ZHKU-NetAutoAuth");
                        ui.label(
                            egui::RichText::new("白云校区 · 校园网自动认证")
                                .color(egui::Color32::GRAY),
                        );
                    });
                });
            });
        // Allocate the upper workspace first. The drawer can only consume the
        // remaining area, so resizing it cannot cover either account card.
        egui::TopBottomPanel::top("workspace")
            .default_height(400.0)
            .min_height(400.0)
            .frame(egui::Frame::new().fill(background).inner_margin(16.0))
            .show(ctx, |ui| {
                self.status_card(ui, ctx);
                ui.add_space(8.0);
                ui.columns(2, |columns| {
                    self.controls_card(&mut columns[0], ctx);
                    self.account_card(&mut columns[1]);
                });
            });
        let drawer = egui::TopBottomPanel::top("log-drawer")
            .exact_height(48.0)
            .frame(
                egui::Frame::new()
                    .fill(background)
                    .inner_margin(egui::Margin::symmetric(20, 10)),
            );
        drawer.show(ctx, |ui| {
            let toggle = ui.add_sized(
                [ui.available_width(), 24.0],
                egui::Button::new("运行日志").frame(false),
            );
            let arrow = egui::Rect::from_center_size(
                egui::pos2(toggle.rect.left() + 14.0, toggle.rect.center().y),
                egui::vec2(18.0, 18.0),
            );
            Self::chevron(ui, arrow, !self.show_logs);
            if toggle.clicked() {
                self.show_logs = !self.show_logs;
                let size = ctx.screen_rect().size();
                let height = if self.show_logs {
                    (size.y + 200.0).max(760.0)
                } else {
                    (size.y - 200.0).max(560.0)
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    840.0,
                    if self.show_logs { 760.0 } else { 560.0 },
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(size.x, height)));
            }
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(background)
                    .inner_margin(if self.show_logs {
                        egui::Margin::symmetric(20, 10)
                    } else {
                        egui::Margin::ZERO
                    }),
            )
            .show(ctx, |ui| {
                if self.show_logs {
                    self.logs_card(ui);
                }
            });
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
        while self.logs.len() > 200 {
            self.logs.pop_front();
        }
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

    fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
        let width = ui.available_width();
        egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(224, 230, 240),
            ))
            .corner_radius(12)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width((width - 34.0).max(100.0));
                content(ui);
            });
    }

    fn status_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("网络状态");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            self.receiver.is_none(),
                            egui::Button::new(if self.receiver.is_some() {
                                "查询中…"
                            } else {
                                "刷新状态"
                            })
                            .min_size(egui::vec2(96.0, 28.0)),
                        )
                        .clicked()
                    {
                        self.launch(Action::Check, ctx);
                    }
                });
            });
            let (text, color) = match self.status.state {
                State::Online => ("已通过认证", egui::Color32::from_rgb(24, 130, 86)),
                State::Offline => ("尚未认证", egui::Color32::from_rgb(175, 105, 20)),
                State::Unknown => ("状态待确认", egui::Color32::from_rgb(100, 110, 125)),
            };
            ui.label(egui::RichText::new(text).size(28.0).color(color))
                .on_hover_text(&self.status.detail);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("所在区域：{}", self.status.region));
                if !self.status.ip.is_empty() {
                    ui.label(format!("IPv4：{}", self.status.ip));
                }
                if !self.status.account.is_empty() {
                    ui.label(format!("在线账号：{}", self.status.account));
                }
            });
        });
    }

    fn input_frame() -> egui::Frame {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(248, 250, 253))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(175, 187, 205),
            ))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(10, 5))
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

    fn account_card(&mut self, ui: &mut egui::Ui) {
        Self::card(ui, |ui| {
            ui.set_min_height(184.0);
            ui.heading("账号配置");
            ui.add_enabled_ui(self.receiver.is_none() && !self.guard, |ui| {
                let mut account_changed = false;
                let mut account_lost_focus = false;
                let input_width = ui.available_width();
                Self::input_frame().show(ui, |ui| {
                    ui.set_width(input_width - 22.0);
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 44.0).max(100.0);
                        let account = ui.add_sized(
                            [width, 28.0],
                            egui::TextEdit::singleline(&mut self.username)
                                .id(egui::Id::new("campus-account"))
                                .frame(false)
                                .hint_text("校园网账号"),
                        );
                        account_changed = account.changed();
                        account_lost_focus = account.lost_focus();
                        ui.allocate_ui_with_layout(
                            egui::vec2(32.0, 28.0),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                let menu = egui::menu::menu_custom_button(
                                    ui,
                                    egui::Button::new(" ")
                                        .frame(false)
                                        .min_size(egui::vec2(32.0, 28.0)),
                                    |ui| {
                                        if let Some(account) = self.saved_username.clone() {
                                            ui.horizontal(|ui| {
                                                if ui
                                                    .selectable_label(
                                                        self.username == account,
                                                        account,
                                                    )
                                                    .clicked()
                                                {
                                                    self.restore_saved_account();
                                                    ui.close_menu();
                                                }
                                                if Self::delete_icon(ui).clicked() {
                                                    self.forget_credentials(ui.ctx());
                                                    ui.close_menu();
                                                }
                                            });
                                        } else {
                                            ui.label("暂无保存的账号");
                                        }
                                    },
                                )
                                .response
                                .on_hover_text("已保存账号");
                                Self::chevron(ui, menu.rect, true);
                            },
                        );
                    });
                });
                let mut password_changed = false;
                let mut password_lost_focus = false;
                Self::input_frame().show(ui, |ui| {
                    ui.set_width(input_width - 22.0);
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 44.0).max(80.0);
                        let password = ui.add_sized(
                            [width, 28.0],
                            egui::TextEdit::singleline(&mut *self.password)
                                .id(egui::Id::new("campus-password"))
                                .frame(false)
                                .password(!self.show_password)
                                .hint_text("密码"),
                        );
                        password_changed = password.changed();
                        password_lost_focus = password.lost_focus();
                        // Do not retain older passwords in the widget's undo history.
                        if let Some(mut state) =
                            egui::text_edit::TextEditState::load(ui.ctx(), password.id)
                        {
                            state.clear_undoer();
                            state.store(ui.ctx(), password.id);
                        }
                        if Self::eye_button(ui, self.show_password).clicked() {
                            self.show_password = !self.show_password;
                        }
                    });
                });
                self.settings_dirty |= account_changed || password_changed;
                if self.settings_dirty && (account_lost_focus || password_lost_focus) {
                    self.persist_settings();
                }
                ui.horizontal_wrapped(|ui| {
                    let remember_changed = ui
                        .checkbox(&mut self.remember_password, "记住密码")
                        .changed();
                    if remember_changed && !self.remember_password {
                        self.auto_login = false;
                    }
                    let configured = !self.username.trim().is_empty() && !self.password.is_empty();
                    let auto_changed = ui
                        .add_enabled(
                            configured || self.auto_login,
                            egui::Checkbox::new(&mut self.auto_login, "自动登录"),
                        )
                        .on_hover_text("下次启动时自动认证，并开启掉线守护")
                        .changed();
                    if auto_changed && self.auto_login {
                        self.remember_password = true;
                    }
                    if remember_changed || auto_changed {
                        self.settings_dirty = true;
                        self.persist_settings();
                    }
                });
            });
        });
    }

    fn chevron(ui: &egui::Ui, rect: egui::Rect, down: bool) {
        let center = rect.center();
        let direction = if down { 1.0 } else { -1.0 };
        let points = vec![
            center + egui::vec2(-4.5, -2.0 * direction),
            center + egui::vec2(0.0, 2.5 * direction),
            center + egui::vec2(4.5, -2.0 * direction),
        ];
        ui.painter().add(egui::Shape::line(
            points,
            egui::Stroke::new(1.3, ui.visuals().text_color()),
        ));
    }

    fn delete_icon(ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
        let visuals = ui.style().interact(&response);
        let stroke = egui::Stroke::new(1.3, visuals.fg_stroke.color);
        let center = rect.center();
        let painter = ui.painter_at(rect);
        if response.hovered() {
            painter.rect_filled(rect, 4, visuals.bg_fill);
        }
        painter.line_segment(
            [
                center + egui::vec2(-4.0, -4.0),
                center + egui::vec2(4.0, 4.0),
            ],
            stroke,
        );
        painter.line_segment(
            [
                center + egui::vec2(-4.0, 4.0),
                center + egui::vec2(4.0, -4.0),
            ],
            stroke,
        );
        response.on_hover_text("删除保存的账号和密码")
    }

    fn eye_button(ui: &mut egui::Ui, visible: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 28.0), egui::Sense::click());
        let visuals = ui.style().interact(&response);
        let painter = ui.painter_at(rect);
        if response.hovered() {
            painter.rect_filled(rect, 5, visuals.bg_fill);
        }
        let center = rect.center();
        let mut points = Vec::new();
        for i in 0..=16 {
            let t = i as f32 / 16.0;
            points.push(
                center + egui::vec2(-11.0 + 22.0 * t, -6.0 * (std::f32::consts::PI * t).sin()),
            );
        }
        for i in (0..=16).rev() {
            let t = i as f32 / 16.0;
            points.push(
                center + egui::vec2(-11.0 + 22.0 * t, 6.0 * (std::f32::consts::PI * t).sin()),
            );
        }
        let stroke = egui::Stroke::new(1.5, visuals.fg_stroke.color);
        painter.add(egui::Shape::closed_line(points, stroke));
        painter.circle_stroke(center, 3.0, stroke);
        if visible {
            painter.line_segment(
                [
                    center + egui::vec2(-10.0, 9.0),
                    center + egui::vec2(10.0, -9.0),
                ],
                stroke,
            );
        }
        response.on_hover_text(if visible {
            "隐藏密码"
        } else {
            "显示密码"
        })
    }

    fn controls_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::card(ui, |ui| {
            ui.set_min_height(184.0);
            ui.heading("连接控制");
            let busy = self.receiver.is_some();
            let configured = !self.username.trim().is_empty() && !self.password.is_empty();
            ui.horizontal_wrapped(|ui| {
                let login =
                    egui::Button::new(egui::RichText::new("立即认证").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(50, 100, 210))
                        .min_size(egui::vec2(120.0, 38.0));
                if ui.add_enabled(!busy && configured, login).clicked() {
                    self.launch(Action::Login, ctx);
                }
                let logout = egui::Button::new(
                    egui::RichText::new("注销连接").color(egui::Color32::from_rgb(180, 65, 65)),
                )
                .min_size(egui::vec2(120.0, 38.0));
                if ui
                    .add_enabled(!busy && self.status.state == State::Online, logout)
                    .on_hover_text("停止自动守护，并注销当前校园网会话；保留本次凭据")
                    .clicked()
                {
                    self.launch(Action::Logout, ctx);
                }
            });
            ui.separator();
            // Stopping guard remains available while a network task is running.
            if ui
                .add_enabled(
                    self.guard || configured,
                    egui::Checkbox::new(&mut self.guard, "自动守护"),
                )
                .on_hover_text(if configured {
                    "开启掉线检查和自动重认证"
                } else {
                    "请先填写校园网账号和密码"
                })
                .changed()
            {
                self.failures = 0;
                self.next_check = Instant::now();
                self.guard_pending = self.guard && busy;
                self.log(if self.guard {
                    "已开启自动守护"
                } else {
                    "已停止自动守护；进行中的网络请求完成后不再重试"
                });
            }
            if self.guard {
                let seconds = self
                    .next_check
                    .saturating_duration_since(Instant::now())
                    .as_secs();
                ui.label(if busy {
                    "正在检查连接…".into()
                } else {
                    format!("下次检查约 {seconds} 秒后")
                });
            } else {
                ui.label("守护已停止");
            }
            if !configured {
                ui.label(
                    egui::RichText::new("填写账号和密码后可开启守护")
                        .color(egui::Color32::from_rgb(150, 105, 40)),
                );
            }
        });
    }

    fn logs_card(&mut self, ui: &mut egui::Ui) {
        Self::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("清空日志").clicked() {
                        self.logs.clear();
                    }
                    ui.checkbox(&mut self.errors_only, "仅看异常");
                });
            });
            let height = ui.available_height().max(40.0);
            egui::ScrollArea::vertical()
                .id_salt("log-scroll")
                .max_height(height)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let mut count = 0;
                    for entry in &self.logs {
                        if self.errors_only && !entry.failed {
                            continue;
                        }
                        count += 1;
                        let seconds = entry.elapsed.as_secs();
                        let label = format!(
                            "[{:02}:{:02}:{:02}] [{}] {}",
                            seconds / 3600,
                            (seconds / 60) % 60,
                            seconds % 60,
                            if entry.failed { "ERROR" } else { "DEBUG" },
                            entry.message
                        );
                        let color = if entry.failed {
                            egui::Color32::from_rgb(180, 65, 65)
                        } else {
                            egui::Color32::from_rgb(70, 85, 105)
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(label)
                                    .monospace()
                                    .size(13.0)
                                    .color(color),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                    }
                    if count == 0 {
                        ui.label(if self.errors_only {
                            "暂无异常记录"
                        } else {
                            "暂无记录"
                        });
                    }
                });
        });
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
            while self.logs.len() > 200 {
                self.logs.pop_front();
            }
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.settings_dirty {
            self.persist_settings();
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                let delay = (10 * 2u64.pow(self.failures)).min(300);
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
        self.draw(ctx);
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "ZHKU-NetAutoAuth",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([880.0, 560.0])
                .with_min_inner_size([840.0, 560.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        App::configure_context(&ctx);
        ctx
    }

    fn draw(
        app: &mut App,
        ctx: &egui::Context,
        height: f32,
        events: Vec<egui::Event>,
    ) -> Vec<egui::Rect> {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(880.0, height),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        );
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::WHITE
                        && rect.stroke.color == egui::Color32::from_rgb(224, 230, 240) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn paired_cards_are_equal_and_busy_state_does_not_resize_status() {
        let ctx = context();
        let mut app = App::empty();
        let idle = draw(&mut app, &ctx, 560.0, vec![]);
        assert_eq!(idle.len(), 3);
        assert!(
            (idle[1].height() - idle[2].height()).abs() < 0.5,
            "unequal cards: {idle:?}"
        );
        let (_sender, receiver) = mpsc::channel();
        app.receiver = Some(receiver);
        let busy = draw(&mut app, &ctx, 560.0, vec![]);
        assert_eq!(busy.len(), 3);
        assert!((idle[0].height() - busy[0].height()).abs() < 0.5);
        assert!((busy[1].height() - busy[2].height()).abs() < 0.5);
        let workspace = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("workspace"))
            .unwrap()
            .rect;
        assert!(
            busy.iter()
                .all(|card| card.bottom() <= workspace.bottom() + 0.5)
        );
    }

    #[test]
    fn drawer_header_cannot_be_dragged_over_workspace() {
        let ctx = context();
        let mut app = App::empty();
        app.show_logs = true;
        for _ in 0..20 {
            app.log("example raw response diagnostic line");
        }
        draw(&mut app, &ctx, 760.0, vec![]);
        let drawer = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("log-drawer"))
            .unwrap()
            .rect;
        let initial_height = drawer.height();
        let divider = egui::pos2(drawer.center().x, drawer.top());
        draw(
            &mut app,
            &ctx,
            760.0,
            vec![
                egui::Event::PointerMoved(divider),
                egui::Event::PointerButton {
                    pos: divider,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        draw(
            &mut app,
            &ctx,
            760.0,
            vec![egui::Event::PointerMoved(egui::pos2(divider.x, 80.0))],
        );
        let workspace = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("workspace"))
            .unwrap()
            .rect;
        let drawer = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("log-drawer"))
            .unwrap()
            .rect;
        assert!(
            drawer.top() >= workspace.bottom() - 0.5,
            "overlap: {workspace:?} {drawer:?}"
        );
        assert!(drawer.bottom() <= 760.5);
        assert!((drawer.height() - initial_height).abs() < 0.5);
        assert!((drawer.top() - workspace.bottom()).abs() < 0.5);
    }

    #[test]
    fn input_frames_have_identical_edges_at_different_window_widths() {
        let ctx = context();
        let mut app = App::empty();
        for width in [840.0, 880.0, 1040.0] {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 560.0),
                    )),
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            );
            let inputs: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == egui::Color32::from_rgb(248, 250, 253)
                            && rect.stroke.color == egui::Color32::from_rgb(175, 187, 205) =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(inputs.len(), 2);
            assert!(
                (inputs[0].left() - inputs[1].left()).abs() < 0.5,
                "{inputs:?}"
            );
            assert!(
                (inputs[0].right() - inputs[1].right()).abs() < 0.5,
                "{inputs:?}"
            );
            let arrow_x = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Path(path)
                        if path.points.len() == 3 && inputs[0].contains(path.points[1]) =>
                    {
                        Some(path.points[1].x)
                    }
                    _ => None,
                })
                .expect("account chevron");
            let eye_x = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Circle(circle)
                        if (circle.radius - 3.0).abs() < 0.1
                            && inputs[1].contains(circle.center) =>
                    {
                        Some(circle.center.x)
                    }
                    _ => None,
                })
                .expect("password eye");
            assert!(
                (arrow_x - eye_x).abs() < 0.5,
                "misaligned icons at width {width}: {arrow_x} != {eye_x}"
            );
        }
    }

    #[test]
    fn expanded_logs_start_below_cards_and_fill_remaining_height() {
        let ctx = context();
        let mut app = App::empty();
        app.show_logs = true;
        for _ in 0..40 {
            app.log("raw response diagnostic example");
        }
        for height in [760.0, 900.0] {
            let cards = draw(&mut app, &ctx, height, vec![]);
            assert_eq!(cards.len(), 4);
            let workspace =
                egui::containers::panel::PanelState::load(&ctx, egui::Id::new("workspace"))
                    .unwrap()
                    .rect;
            let drawer =
                egui::containers::panel::PanelState::load(&ctx, egui::Id::new("log-drawer"))
                    .unwrap()
                    .rect;
            assert!((drawer.top() - workspace.bottom()).abs() < 0.5);
            assert!(
                (cards[3].top() - drawer.bottom() - 10.0).abs() < 1.0,
                "gap: {cards:?} {drawer:?}"
            );
            assert!(
                (height - cards[3].bottom() - 10.0).abs() < 1.0,
                "unused space: {cards:?}"
            );
        }
    }
}
