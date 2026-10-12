use super::Command;
use crate::theme;
use std::sync::mpsc::Sender;
use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub(super) struct NativeTray {
    icon: TrayIcon,
    network: MenuItem,
    guard: MenuItem,
    last_status: String,
}

impl NativeTray {
    pub fn new(wake: super::wake::WakeSignal, sender: Sender<Command>) -> Result<Self, String> {
        let menu = Menu::new();
        let network = MenuItem::new("网络：正在查询", false, None);
        let guard = MenuItem::new("自动守护：已停止", false, None);
        let open = MenuItem::new("打开窗口", true, None);
        let exit = MenuItem::new("退出程序", true, None);
        menu.append_items(&[
            &network,
            &guard,
            &PredefinedMenuItem::separator(),
            &open,
            &exit,
        ])
        .map_err(|_| "无法创建托盘菜单")?;
        let image = theme::window_icon();
        let image = Icon::from_rgba(image.rgba, image.width, image.height)
            .map_err(|_| "无法生成托盘图标")?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("ZHKU-NetAutoAuth")
            .with_icon(image)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|_| "无法创建系统托盘，关闭窗口将退出程序")?;

        // Native callbacks wake egui even when the main window is hidden.
        let open_id = open.id().clone();
        let exit_id = exit.id().clone();
        let menu_wake = wake.clone();
        let menu_sender = sender.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let command = if event.id == open_id {
                Some(Command::Show)
            } else if event.id == exit_id {
                Some(Command::Exit)
            } else {
                None
            };
            if let Some(command) = command {
                let _ = menu_sender.send(command);
                menu_wake.wake();
            }
        }));
        let id = icon.id().clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if matches!(event, TrayIconEvent::DoubleClick { id: event_id, button: MouseButton::Left, .. } if event_id == id)
            {
                let _ = sender.send(Command::Show);
                wake.wake();
            }
        }));
        // Keep the native icon and menu items alive on the GUI thread.
        Ok(Self {
            icon,
            network,
            guard,
            last_status: String::new(),
        })
    }

    pub fn sync(&mut self, network: &str, region: &str, guarding: bool) {
        let guard = if guarding { "运行中" } else { "已停止" };
        let status = format!("ZHKU-NetAutoAuth\n{network} · {region}\n自动守护：{guard}");
        if status == self.last_status {
            return;
        }
        self.network.set_text(format!("网络：{network} · {region}"));
        self.guard.set_text(format!("自动守护：{guard}"));
        // Deliberately exclude the account, IP and MAC from the public tooltip.
        let _ = self.icon.set_tooltip(Some(&status));
        self.last_status = status;
    }
}

// Tray/menu callbacks are process-global in tray-icon. Remove our callbacks on exit.
impl Drop for NativeTray {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}
