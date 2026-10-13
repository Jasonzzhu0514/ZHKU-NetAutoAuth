#[cfg(windows)]
mod native;
mod preferences;
#[cfg(windows)]
mod tray;
mod ui;
#[cfg(windows)]
mod wake;

use crate::portal::Status;
use eframe::egui;
use preferences::{CloseBehavior, Preferences};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(super) enum Command {
    Show,
    Exit,
}

#[derive(Default)]
pub(crate) struct Desktop {
    preferences: Preferences,
    file: Option<PathBuf>,
    #[cfg(windows)]
    native: Option<native::NativeDesktop>,
    hidden: bool,
    exiting: bool,
    show_settings: bool,
    show_notice: bool,
    save_error: Option<String>,
}

impl Desktop {
    pub fn initialize(&mut self, cc: &eframe::CreationContext<'_>) -> Vec<String> {
        let mut errors = Vec::new();
        match crate::paths::desktop_preferences() {
            Ok(file) => {
                match Preferences::load(&file) {
                    Ok(prefs) => self.preferences = prefs,
                    Err(e) => errors.push(e),
                }
                self.file = Some(file);
            }
            Err(e) => errors.push(e),
        }
        #[cfg(windows)]
        {
            match native::NativeDesktop::new(cc) {
                Ok(native) => self.native = Some(native),
                Err(error) => errors.push(error),
            }
        }
        self.show_notice = !self.preferences.notice_acknowledged;
        errors
    }

    pub fn ready(&self) -> bool {
        #[cfg(windows)]
        {
            self.native.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub fn open_settings(&mut self) {
        self.show_settings = true;
    }
    pub fn auto_check_updates(&self) -> bool {
        self.preferences.auto_check_updates
    }
    pub fn exit(&mut self, ctx: &egui::Context) {
        self.command(Command::Exit, ctx);
    }

    fn save(&mut self) {
        self.save_error = match &self.file {
            Some(file) => self.preferences.save(file).err(),
            None => Some("无法确定设置目录，本次选择不会保存".into()),
        };
    }

    fn command(&mut self, command: Command, ctx: &egui::Context) {
        match command {
            Command::Show if !self.exiting => {
                self.hidden = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            Command::Exit => {
                self.exiting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, status: &Status, guarding: bool) -> bool {
        #[cfg(windows)]
        let commands = self
            .native
            .as_ref()
            .map(|native| native.take_commands())
            .unwrap_or_default();
        #[cfg(not(windows))]
        let commands: Vec<Command> = Vec::new();
        for command in commands {
            self.command(command, ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.exiting {
            self.close_requested(ctx, self.ready());
        }
        #[cfg(windows)]
        if let Some(native) = &mut self.native {
            let background =
                self.hidden || ctx.input(|input| input.viewport().minimized.unwrap_or(false));
            native.sync(
                status,
                guarding && !self.exiting,
                background && !self.exiting,
            );
        }
        self.exiting
    }

    fn close_requested(&mut self, ctx: &egui::Context, tray_ready: bool) {
        if self.exiting {
            return;
        }
        if tray_ready && self.show_notice {
            // A first-use close must not hide the explanation before it is read.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        } else if tray_ready && self.preferences.close == CloseBehavior::Tray {
            self.hidden = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            // Never leave an inaccessible background process if tray creation failed.
            self.exiting = true;
        }
    }

    pub fn shutdown(&mut self) {
        #[cfg(windows)]
        {
            self.native = None;
        }
    }
}

#[cfg(test)]
mod tests;
