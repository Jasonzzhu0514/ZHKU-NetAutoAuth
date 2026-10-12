use eframe::egui;

pub const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(243, 244, 246);
pub const CARD: egui::Color32 = egui::Color32::WHITE;
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(209, 213, 219);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(29, 29, 31);
pub const SECONDARY: egui::Color32 = egui::Color32::from_rgb(110, 110, 115);
pub const BLUE: egui::Color32 = egui::Color32::from_rgb(0, 122, 255);
pub const GREEN: egui::Color32 = egui::Color32::from_rgb(36, 138, 61);
pub const RED: egui::Color32 = egui::Color32::from_rgb(211, 73, 67);
pub const DIVIDER: egui::Color32 = egui::Color32::from_rgb(229, 231, 235);
pub const INPUT: egui::Color32 = egui::Color32::from_rgb(249, 250, 251);
pub const PAGE_MARGIN: f32 = 28.0;
pub const GAP: f32 = 20.0;
pub const CARD_PADDING: f32 = 20.0;
pub const PAIR_HEIGHT: f32 = 244.0;
pub const STACK_WIDTH: f32 = 760.0;

pub fn configure(ctx: &egui::Context) {
    configure_fonts(ctx);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size.y = 24.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, DIVIDER);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
        widget.fg_stroke = egui::Stroke::new(1.0, TEXT);
        widget.bg_stroke = egui::Stroke::new(1.0, BORDER);
    }
    style.visuals.widgets.inactive.weak_bg_fill = CARD;
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(240, 246, 255);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, BLUE);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(225, 236, 252);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, BLUE);
    style.visuals.selection.bg_fill = BLUE;
    ctx.set_style(style);
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut bold = Vec::new();
    if let Some(windows) = std::env::var_os("WINDIR") {
        let directory = std::path::PathBuf::from(windows).join("Fonts");
        for file in ["msyh.ttc", "simhei.ttf", "simsun.ttc"] {
            if let Ok(bytes) = std::fs::read(directory.join(file)) {
                fonts
                    .font_data
                    .insert("chinese".into(), egui::FontData::from_owned(bytes).into());
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "chinese".into());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("chinese".into());
                break;
            }
        }
        for (name, file) in [
            ("windows", "segoeui.ttf"),
            ("windows-bold", "segoeuib.ttf"),
            ("chinese-bold", "msyhbd.ttc"),
        ] {
            if let Ok(bytes) = std::fs::read(directory.join(file)) {
                fonts
                    .font_data
                    .insert(name.into(), egui::FontData::from_owned(bytes).into());
                if name == "windows" {
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, name.into());
                } else {
                    bold.push(name.into());
                }
            }
        }
    }
    bold.extend(
        fonts.families[&egui::FontFamily::Proportional]
            .iter()
            .cloned(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("WindowsBold".into()), bold);
    ctx.set_fonts(fonts);
}

pub fn heading_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("WindowsBold".into()))
}

pub fn window_icon() -> egui::IconData {
    let mut rgba = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let node = [(16_i32, 8_i32), (8, 24), (24, 24)]
                .iter()
                .any(|(cx, cy)| (x as i32 - cx).pow(2) + (y as i32 - cy).pow(2) <= 16);
            let line = ((15..=16).contains(&x) && (8..=20).contains(&y))
                || ((8..=24).contains(&x) && (19..=20).contains(&y))
                || ([8, 24].contains(&x) && (20..=24).contains(&y));
            if node || line {
                rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[0, 122, 255, 255]);
            }
        }
    }
    egui::IconData {
        rgba,
        width: 32,
        height: 32,
    }
}
