use super::*;

impl App {
    pub(super) fn network_icon(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
        let painter = ui.painter();
        let center = rect.center();
        let stroke = egui::Stroke::new(1.5, color);
        let top = center + egui::vec2(0.0, -6.0);
        let left = center + egui::vec2(-8.0, 6.0);
        let right = center + egui::vec2(8.0, 6.0);
        painter.line_segment([top, center], stroke);
        painter.line_segment([left, right], stroke);
        painter.line_segment([center, center + egui::vec2(0.0, 6.0)], stroke);
        for point in [top, left, right] {
            painter.rect_filled(
                egui::Rect::from_center_size(point, egui::vec2(7.0, 6.0)),
                1,
                color,
            );
        }
    }

    pub(super) fn refresh_icon(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
        let center = rect.center();
        let radius = rect.width() / 2.0;
        let points: Vec<_> = (0..=20)
            .map(|i| {
                let angle = std::f32::consts::PI * (0.2 + 1.6 * i as f32 / 20.0);
                center + egui::vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        let end = *points.last().expect("arc points");
        ui.painter()
            .add(egui::Shape::line(points, egui::Stroke::new(1.3, color)));
        ui.painter().line_segment(
            [end, end + egui::vec2(-4.0, -1.0)],
            egui::Stroke::new(1.3, color),
        );
    }

    pub(super) fn guard_switch(
        ui: &mut egui::Ui,
        value: &mut bool,
        enabled: bool,
    ) -> egui::Response {
        let (rect, mut response) =
            ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::click());
        if enabled
            && (response.clicked()
                || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Space))))
        {
            *value = !*value;
            response.mark_changed();
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, *value, "自动守护")
        });
        let fill = if !enabled {
            t::DIVIDER
        } else if *value {
            t::BLUE
        } else if response.hovered() {
            t::BORDER
        } else {
            egui::Color32::from_rgb(188, 192, 199)
        };
        ui.painter().rect_filled(rect, 12, fill);
        let center = egui::pos2(
            if *value {
                rect.right() - 12.0
            } else {
                rect.left() + 12.0
            },
            rect.center().y,
        );
        ui.painter().circle_filled(center, 9.0, t::CARD);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                12,
                egui::Stroke::new(1.0, t::BLUE),
                egui::StrokeKind::Outside,
            );
        }
        response
    }
    pub(super) fn chevron(ui: &egui::Ui, rect: egui::Rect, down: bool) {
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

    pub(super) fn delete_icon(ui: &mut egui::Ui) -> egui::Response {
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

    pub(super) fn eye_button(ui: &mut egui::Ui, visible: bool) -> egui::Response {
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
}
