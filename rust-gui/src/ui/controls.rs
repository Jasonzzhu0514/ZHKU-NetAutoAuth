use super::*;
use std::time::Instant;

impl App {
    pub(super) fn controls_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::card(ui, t::PAIR_HEIGHT, |ui| {
            Self::heading(ui, "连接控制");
            ui.add_space(8.0);
            let busy = self.receiver.is_some();
            let configured = !self.username.trim().is_empty() && !self.password.is_empty();
            let button_width = (ui.available_width() - 12.0) / 2.0;
            ui.horizontal(|ui| {
                let login = ui
                    .scope(|ui| {
                        ui.visuals_mut().widgets.inactive.weak_bg_fill = t::BLUE;
                        ui.visuals_mut().widgets.hovered.weak_bg_fill =
                            egui::Color32::from_rgb(0, 108, 230);
                        ui.visuals_mut().widgets.active.weak_bg_fill =
                            egui::Color32::from_rgb(0, 91, 196);
                        stable_busy_button(
                            ui,
                            configured,
                            busy,
                            egui::Button::new(egui::RichText::new("立即认证").color(t::CARD))
                                .stroke(egui::Stroke::new(1.0, t::BLUE))
                                .min_size(egui::vec2(button_width, 36.0)),
                        )
                    })
                    .inner;
                if !busy && login.clicked() {
                    self.launch(Action::Login, ctx);
                }
                let logout = ui
                    .scope(|ui| {
                        stable_busy_button(
                            ui,
                            self.status.state == State::Online,
                            busy,
                            egui::Button::new(egui::RichText::new("注销连接").color(t::RED))
                                .min_size(egui::vec2(button_width, 36.0)),
                        )
                    })
                    .inner
                    .on_hover_text("停止自动守护，并注销当前校园网会话；保留本次凭据");
                if !busy && logout.clicked() {
                    self.launch(Action::Logout, ctx);
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let label_width = (ui.available_width() - 54.0).max(100.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(label_width, 40.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label("自动守护");
                        let detail = if self.guard {
                            "守护运行中"
                        } else {
                            "守护已停止"
                        };
                        ui.label(egui::RichText::new(detail).size(12.0).color(t::SECONDARY));
                    },
                );
                let enabled = configured || self.guard;
                let response =
                    Self::guard_switch(ui, &mut self.guard, enabled).on_hover_text(if configured {
                        "开启掉线检查和自动重认证"
                    } else {
                        "请先填写校园网账号和密码"
                    });
                if response.changed() {
                    self.failures = 0;
                    self.next_check = Instant::now();
                    self.guard_pending = self.guard && busy;
                    self.log(if self.guard {
                        "已开启自动守护"
                    } else {
                        "已停止自动守护；进行中的网络请求完成后不再重试"
                    });
                }
            });
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("开启后，断线时将自动重新认证。")
                    .size(12.0)
                    .color(t::SECONDARY),
            );
        });
    }
}

// Network work still blocks interaction, but does not briefly fade the controls.
// Missing credentials/offline state keep the normal disabled appearance.
fn stable_busy_button(
    ui: &mut egui::Ui,
    available: bool,
    busy: bool,
    button: egui::Button<'_>,
) -> egui::Response {
    let response = ui.add_enabled(available, button);
    if busy && available {
        response.on_hover_cursor(egui::CursorIcon::Wait)
    } else {
        response
    }
}
