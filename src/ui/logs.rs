use super::*;

impl App {
    fn log_viewport(&mut self, ui: &mut egui::Ui, viewport: egui::Rect) {
        use super::log_view::{Kind, ROW_GAP};
        ui.spacing_mut().item_spacing.y = 0.0;
        let message_width = (ui.available_width() - 136.0).max(60.0);
        self.log_view.sync(
            &self.logs,
            self.errors_only,
            message_width,
            ui.ctx().pixels_per_point(),
        );
        if self.log_view.is_empty() {
            ui.label(
                egui::RichText::new(if self.errors_only {
                    "暂无异常记录"
                } else {
                    "暂无记录"
                })
                .color(t::SECONDARY),
            );
            return;
        }
        let range = self
            .log_view
            .visible_rows(viewport.top(), viewport.bottom());
        ui.add_space(self.log_view.offset(range.start));
        let mut changed = None;
        for index in range.clone() {
            let row = self.log_view.row(index);
            let entry = &self.logs[row.entry];
            let (level, color) = match row.kind {
                Kind::Error => ("ERROR", t::RED),
                Kind::Info => ("INFO", t::GREEN),
                Kind::Debug => ("DEBUG", t::SECONDARY),
            };
            let first = row.bytes.start == 0;
            let seconds = entry.elapsed.as_secs();
            let time = if first {
                format!(
                    "{:02}:{:02}:{:02}",
                    seconds / 3600,
                    (seconds / 60) % 60,
                    seconds % 60
                )
            } else {
                String::new()
            };
            let rendered = ui.horizontal_top(|ui| {
                ui.add_sized(
                    [64.0, 16.0],
                    egui::Label::new(
                        egui::RichText::new(time)
                            .monospace()
                            .size(12.0)
                            .color(t::SECONDARY),
                    ),
                );
                ui.add_sized(
                    [48.0, 16.0],
                    egui::Label::new(
                        egui::RichText::new(if first { level } else { "" })
                            .monospace()
                            .size(12.0)
                            .color(color),
                    ),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            entry.message[row.bytes.clone()].trim_end_matches('\n'),
                        )
                        .size(13.0)
                        .color(color),
                    )
                    .selectable(true)
                    .wrap(),
                )
                .context_menu(|ui| {
                    if ui.button("复制完整记录").clicked() {
                        ui.ctx().copy_text(entry.message.clone());
                        ui.close_menu();
                    }
                });
            });
            if self
                .log_view
                .record_height(index, rendered.response.rect.height())
            {
                changed = Some(changed.map_or(index, |first: usize| first.min(index)));
            }
            ui.add_space(ROW_GAP);
        }
        self.log_view.finish_measurements(changed);
        ui.add_space((self.log_view.total_height() - self.log_view.offset(range.end)).max(0.0));
    }

    pub(super) fn logs_card(&mut self, ui: &mut egui::Ui, height: f32) {
        let width = ui.available_width();
        egui::Frame::new()
            .fill(t::CARD)
            .stroke(egui::Stroke::new(1.0, t::BORDER))
            .corner_radius(8)
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                if !self.show_logs {
                    // Index newly received messages while folded. No glyph
                    // shaping or text widgets are created for hidden logs.
                    self.log_view.sync(
                        &self.logs,
                        self.errors_only,
                        (ui.available_width() - 176.0).max(60.0),
                        ui.ctx().pixels_per_point(),
                    );
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        ui.spacing_mut().button_padding.y = 2.0;
                        ui.horizontal(|ui| {
                            let toggle = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("     运行日志")
                                        .font(t::heading_font(15.0))
                                        .strong(),
                                )
                                .frame(false),
                            );
                            Self::chevron(
                                ui,
                                egui::Rect::from_center_size(
                                    toggle.rect.left_center() + egui::vec2(8.0, 0.0),
                                    egui::vec2(16.0, 16.0),
                                ),
                                !self.show_logs,
                            );
                            if toggle.clicked() {
                                self.show_logs = !self.show_logs;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("清空日志").color(t::SECONDARY),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        self.logs.clear();
                                        self.log_view.clear();
                                    }
                                    ui.checkbox(&mut self.errors_only, "仅看异常");
                                },
                            );
                        });
                    });
                if self.show_logs {
                    ui.separator();
                    egui::Frame::new()
                        .fill(t::INPUT)
                        .inner_margin(egui::Margin::symmetric(20, 16))
                        .show(ui, |ui| {
                            let content_height = (height - 88.0).max(80.0);
                            egui::ScrollArea::vertical()
                                .id_salt("log-scroll")
                                .max_height(content_height)
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                )
                                .stick_to_bottom(true)
                                .show_viewport(ui, |ui, viewport| self.log_viewport(ui, viewport));
                        });
                }
            });
    }
}
