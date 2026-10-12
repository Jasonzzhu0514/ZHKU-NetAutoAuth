use super::*;

impl App {
    pub(super) fn account_card(&mut self, ui: &mut egui::Ui) {
        Self::card(ui, t::PAIR_HEIGHT, |ui| {
            Self::heading(ui, "账号配置");
            ui.add_space(8.0);
            ui.add_enabled_ui(self.receiver.is_none() && !self.guard, |ui| {
                let label_width = 44.0;
                let mut account_changed = false;
                let mut account_lost_focus = false;
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, 38.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label("账号");
                        },
                    );
                    let width = ui.available_width();
                    let field = Self::input_frame().show(ui, |ui| {
                        ui.set_width(width - 22.0);
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 40.0).max(80.0);
                            let edit = ui.add_sized(
                                [width, 28.0],
                                egui::TextEdit::singleline(&mut self.username)
                                    .id(egui::Id::new("campus-account"))
                                    .frame(false)
                                    .hint_text("校园网账号"),
                            );
                            account_changed = edit.changed();
                            account_lost_focus = edit.lost_focus();
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
                        ui.memory(|memory| memory.has_focus(egui::Id::new("campus-account")))
                    });
                    if field.inner {
                        ui.painter().rect_stroke(
                            field.response.rect,
                            6,
                            egui::Stroke::new(1.0, t::BLUE),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
                let mut password_changed = false;
                let mut password_lost_focus = false;
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, 38.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label("密码");
                        },
                    );
                    let width = ui.available_width();
                    let field = Self::input_frame().show(ui, |ui| {
                        ui.set_width(width - 22.0);
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 40.0).max(80.0);
                            let edit = ui.add_sized(
                                [width, 28.0],
                                egui::TextEdit::singleline(&mut *self.password)
                                    .id(egui::Id::new("campus-password"))
                                    .frame(false)
                                    .password(!self.show_password)
                                    .hint_text("密码"),
                            );
                            password_changed = edit.changed();
                            password_lost_focus = edit.lost_focus();
                            if let Some(mut state) =
                                egui::text_edit::TextEditState::load(ui.ctx(), edit.id)
                            {
                                state.clear_undoer();
                                state.store(ui.ctx(), edit.id);
                            }
                            if Self::eye_button(ui, self.show_password).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });
                        ui.memory(|memory| memory.has_focus(egui::Id::new("campus-password")))
                    });
                    if field.inner {
                        ui.painter().rect_stroke(
                            field.response.rect,
                            6,
                            egui::Stroke::new(1.0, t::BLUE),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
                self.settings_dirty |= account_changed || password_changed;
                if self.settings_dirty && (account_lost_focus || password_lost_focus) {
                    self.persist_settings();
                }
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(label_width);
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
}
