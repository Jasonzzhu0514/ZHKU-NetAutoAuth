use super::*;
use crate::theme;

impl Updater {
    pub fn notice(&mut self, ctx: &egui::Context) {
        if self.notify
            && let Some(release) = &self.release
        {
            egui::Window::new("发现新版本")
                .id(egui::Id::new("update-notification"))
                .collapsible(false)
                .resizable(false)
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.label(format!("ZHKU-NetAutoAuth {} 已发布", release.version));
                    ui.horizontal(|ui| {
                        if ui.button("查看更新").clicked() {
                            self.settings_requested = true;
                            self.notify = false;
                        }
                        if ui.button("稍后").clicked() {
                            self.notify = false;
                        }
                    });
                });
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("软件更新").strong());
        let edition = if self.mode == InstallMode::Installed {
            "安装版"
        } else {
            "便携版"
        };
        ui.label(format!(
            "当前版本 {} · {edition}",
            env!("CARGO_PKG_VERSION")
        ));
        ui.label(
            egui::RichText::new(&self.message)
                .size(13.0)
                .color(if self.failed {
                    theme::RED
                } else {
                    theme::SECONDARY
                }),
        );
        if self.downloading {
            let total = self.release.as_ref().map_or(1, |release| release.size);
            ui.add(
                egui::ProgressBar::new((self.bytes as f64 / total as f64) as f32).show_percentage(),
            );
            if ui.button("取消下载").clicked() {
                self.cancel.store(true, Ordering::Relaxed);
                self.message = "正在取消下载…".into();
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.receiver.is_none(), egui::Button::new("检查更新"))
                .clicked()
            {
                self.check(ui.ctx(), false);
            }
            if self.release.is_some() && self.receiver.is_none() {
                if self.prepared.is_some() {
                    if ui.button("安装并重启").clicked() {
                        self.confirm_install = true;
                    }
                } else if ui.button("下载更新").clicked() {
                    self.download(ui.ctx());
                }
            }
            ui.hyperlink_to("发布页面", releases::RELEASES_URL);
        });
        if let Some(release) = &self.release {
            ui.hyperlink_to(format!("{} 更新说明", release.version), &release.page);
            egui::ScrollArea::vertical()
                .id_salt("update-notes")
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(&release.notes).wrap().selectable(true));
                });
        }
        if self.confirm_install {
            egui::Modal::new(egui::Id::new("confirm-update-install")).show(ui.ctx(),|ui| {
                ui.set_max_width(350.0);
                ui.label(egui::RichText::new("安装更新并重启？").size(17.0).strong());
                ui.label("安装期间会暂时停止守护，不会注销校园网。现有配置会保留；未记住的密码需要重新输入。");
                ui.horizontal(|ui| {
                    if ui.button("确认安装").clicked() {self.install_requested=true;self.confirm_install=false;}
                    if ui.button("取消").clicked() {self.confirm_install=false;}
                });
            });
        }
    }
}
