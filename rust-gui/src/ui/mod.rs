mod account;
mod controls;
mod icons;
pub(super) mod log_view;
mod logs;
mod status;

use super::{Action, App};
use crate::{portal::State, theme as t};
use eframe::egui;

impl App {
    pub(super) fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(t::BACKGROUND)
                    .inner_margin(t::PAGE_MARGIN),
            )
            .show(ctx, |ui| {
                // Only compact/short windows need page scrolling; logs always
                // have their own scroll area. No absolute screen positions.
                egui::ScrollArea::vertical()
                    .id_salt("page-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            ui.label(
                                egui::RichText::new("校园网自动认证")
                                    .size(22.0)
                                    .font(t::heading_font(22.0))
                                    .strong()
                                    .color(t::TEXT),
                            );
                        });
                        ui.add_space(8.0);
                        self.status_card(ui, ctx);
                        ui.add_space(8.0);
                        let width = ui.available_width();
                        if width >= t::STACK_WIDTH {
                            let left = (width - t::GAP) * 0.4;
                            let right = width - t::GAP - left;
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = t::GAP;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(left, t::PAIR_HEIGHT),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 12.0;
                                        self.controls_card(ui, ctx);
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    egui::vec2(right, t::PAIR_HEIGHT),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 12.0;
                                        self.account_card(ui);
                                    },
                                );
                            });
                        } else {
                            self.controls_card(ui, ctx);
                            ui.add_space(8.0);
                            self.account_card(ui);
                        }
                        ui.add_space(8.0);
                        let log_height = if self.show_logs {
                            ui.available_height().max(180.0)
                        } else {
                            52.0
                        };
                        self.logs_card(ui, log_height);
                    });
            });
    }

    pub(super) fn card(ui: &mut egui::Ui, height: f32, content: impl FnOnce(&mut egui::Ui)) {
        let width = ui.available_width();
        egui::Frame::new()
            .fill(t::CARD)
            .stroke(egui::Stroke::new(1.0, t::BORDER))
            .corner_radius(8)
            .inner_margin(t::CARD_PADDING)
            .show(ui, |ui| {
                ui.set_width((width - 42.0).max(80.0));
                ui.set_min_height((height - 42.0).max(0.0));
                content(ui);
            });
    }

    pub(super) fn input_frame() -> egui::Frame {
        egui::Frame::new()
            .fill(t::INPUT)
            .stroke(egui::Stroke::new(1.0, t::BORDER))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 4))
    }

    pub(super) fn heading(ui: &mut egui::Ui, text: &str) {
        ui.label(
            egui::RichText::new(text)
                .font(t::heading_font(15.0))
                .strong()
                .color(t::TEXT),
        );
    }
}
