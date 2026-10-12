use super::*;

impl App {
    pub(super) fn status_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::card(ui, 134.0, |ui| {
            let refresh_width = 124.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 44.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let refresh = ui
                        .add_enabled_ui(self.receiver.is_none(), |ui| {
                            ui.add_sized(
                                egui::vec2(refresh_width, 36.0),
                                egui::Button::new(if self.receiver.is_some() {
                                    "     查询中…"
                                } else {
                                    "     刷新状态"
                                })
                                .fill(t::CARD)
                                .stroke(egui::Stroke::new(1.0, t::BORDER)),
                            )
                        })
                        .inner;
                    Self::refresh_icon(
                        ui,
                        egui::Rect::from_center_size(
                            refresh.rect.left_center() + egui::vec2(18.0, 0.0),
                            egui::vec2(14.0, 14.0),
                        ),
                        if self.receiver.is_none() {
                            t::SECONDARY
                        } else {
                            t::BORDER
                        },
                    );
                    if refresh.clicked() {
                        self.launch(Action::Check, ctx);
                    }
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 44.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let (text, color) = match self.status.state {
                                State::Online => ("已通过认证", t::GREEN),
                                State::Offline => ("尚未认证", t::SECONDARY),
                                State::Unknown => ("状态待确认", t::SECONDARY),
                            };
                            let (icon, _) = ui
                                .allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                icon,
                                8,
                                egui::Color32::from_rgb(232, 245, 235),
                            );
                            Self::network_icon(ui, icon.shrink(9.0), t::GREEN);
                            let label_width = ui.available_width();
                            ui.allocate_ui_with_layout(
                                egui::vec2(label_width, 44.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.spacing_mut().item_spacing.y = 3.0;
                                    ui.label(
                                        egui::RichText::new("网络状态")
                                            .size(13.0)
                                            .color(t::SECONDARY),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(text)
                                                .font(t::heading_font(20.0))
                                                .strong()
                                                .color(color),
                                        )
                                        .on_hover_text(&self.status.detail);
                                        let (dot, _) = ui.allocate_exact_size(
                                            egui::vec2(8.0, 8.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(dot.center(), 3.0, color);
                                    });
                                },
                            );
                        },
                    );
                },
            );
            ui.add_space(2.0);
            let mac = display_mac(&self.status.online_mac);
            let vlan = match (self.status.cvid.as_str(), self.status.pvid.as_str()) {
                ("", "") => String::new(),
                ("", pvid) => pvid.into(),
                (cvid, "") => cvid.into(),
                (cvid, pvid) => format!("{cvid}/{pvid}"),
            };
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 24.0;
                let mut fields = vec![
                    ("所在区域", self.status.region),
                    ("IPv4 地址", self.status.ip.as_str()),
                    ("在线账号", self.status.account.as_str()),
                ];
                if !mac.is_empty() {
                    fields.push(("在线 MAC", mac.as_str()));
                }
                if !vlan.is_empty() {
                    fields.push(("VLAN", vlan.as_str()));
                }
                for (label, value) in fields {
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        label,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(12.0),
                            color: t::SECONDARY,
                            ..Default::default()
                        },
                    );
                    job.append(
                        if value.is_empty() { "—" } else { value },
                        8.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(13.0),
                            color: t::TEXT,
                            ..Default::default()
                        },
                    );
                    let response = ui.add(egui::Label::new(job).wrap());
                    if label == "VLAN" {
                        response.on_hover_text(
                            "认证服务器返回的 cvid / pvid；这些值可能随接入位置变化",
                        );
                    }
                }
            });
        });
    }
}

fn display_mac(value: &str) -> String {
    if value.len() == 12 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        value
            .as_bytes()
            .chunks(2)
            .map(|pair| String::from_utf8_lossy(pair).to_uppercase())
            .collect::<Vec<_>>()
            .join(":")
    } else {
        value.to_owned()
    }
}
