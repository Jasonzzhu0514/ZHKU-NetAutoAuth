use super::*;
use crate::theme;

impl Desktop {
    pub fn ui(&mut self, ctx: &egui::Context, updater: &mut crate::updates::Updater) {
        if self.show_settings {
            let mut open = true;
            egui::Window::new("设置")
                .id(egui::Id::new("desktop-settings"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("关闭窗口时").strong());
                    let before = self.preferences.close;
                    self.close_options(ui);
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(
                        "驻留托盘时，已开启的自动守护会继续运行。托盘菜单中的「退出程序」会结束程序并停止守护。"
                    ).size(12.0).color(theme::SECONDARY));
                    if before != self.preferences.close { self.save(); }
                    self.save_error(ui);
                    ui.separator();
                    if ui.checkbox(&mut self.preferences.auto_check_updates,"认证联网后自动检查更新").changed() {self.save();}
                    updater.ui(ui);
                });
            self.show_settings = open;
        }
        if self.show_notice {
            egui::Modal::new(egui::Id::new("desktop-first-use")).show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.label(egui::RichText::new("后台运行与关闭窗口").size(18.0).strong());
                ui.add_space(8.0);
                ui.label("默认点击窗口 × 会隐藏到系统托盘，已开启的自动守护会继续运行。");
                ui.label("双击托盘图标可恢复窗口，右键菜单可以查看状态或退出程序。");
                ui.add_space(8.0);
                self.close_options(ui);
                ui.label(egui::RichText::new("之后可在页面右上角「设置」中更改。隐藏窗口不会自动开启守护，也不会注销校园网。").size(12.0).color(theme::SECONDARY));
                ui.add_space(8.0);
                if ui.button("知道了").clicked() {
                    self.preferences.notice_acknowledged = true;
                    self.save();
                    self.show_notice = false;
                    // Keep a failed-save explanation accessible after dismissal.
                    if self.save_error.is_some() { self.show_settings = true; }
                }
                self.save_error(ui);
            });
        }
    }

    fn close_options(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.ready(), |ui| {
            ui.radio_value(
                &mut self.preferences.close,
                CloseBehavior::Tray,
                "驻留系统托盘，继续运行",
            );
        });
        ui.radio_value(
            &mut self.preferences.close,
            CloseBehavior::Exit,
            "退出程序，停止守护",
        );
        if !self.ready() {
            ui.label(egui::RichText::new("系统托盘不可用，关闭窗口将直接退出。").color(theme::RED));
        }
    }

    fn save_error(&self, ui: &mut egui::Ui) {
        if let Some(error) = &self.save_error {
            ui.label(egui::RichText::new(error).size(12.0).color(theme::RED));
        }
    }
}
