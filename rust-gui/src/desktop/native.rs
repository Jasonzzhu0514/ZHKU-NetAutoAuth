use super::{Command, tray::NativeTray, wake::NativeWake};
use crate::portal::{State, Status};
use std::sync::mpsc;

/// These resources are created together. Field order removes callbacks and the
/// icon before stopping the wake thread, then releases the event receiver.
pub(super) struct NativeDesktop {
    tray: NativeTray,
    wake: NativeWake,
    commands: mpsc::Receiver<Command>,
}

impl NativeDesktop {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let wake = NativeWake::new(cc)?;
        let (sender, commands) = mpsc::channel();
        let tray = NativeTray::new(wake.signal(), sender)?;
        Ok(Self {
            tray,
            wake,
            commands,
        })
    }

    pub fn take_commands(&self) -> Vec<Command> {
        self.commands.try_iter().collect()
    }

    pub fn sync(&mut self, status: &Status, guarding: bool, background: bool) {
        self.wake.set_background(background);
        let label = match status.state {
            State::Online => "已通过认证",
            State::Offline => "尚未认证",
            State::Unknown => "状态待确认",
        };
        self.tray.sync(label, status.region, guarding);
    }
}
