use eframe::{CreationContext, egui};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_PAINT};

#[derive(Clone)]
pub(super) struct WakeSignal {
    ctx: egui::Context,
    hwnd: usize,
    background: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

impl WakeSignal {
    pub fn wake(&self) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        self.ctx.request_repaint();
        // winit's request_redraw uses RedrawWindow, which does not deliver paint
        // events to a hidden HWND. Explicitly queue one on the GUI thread.
        // No UI or network state is touched by the timer/callback thread.
        unsafe {
            PostMessageW(self.hwnd as _, WM_PAINT, 0, 0);
        }
    }
}

pub(super) struct NativeWake {
    signal: WakeSignal,
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl NativeWake {
    pub fn new(cc: &CreationContext<'_>) -> Result<Self, String> {
        let RawWindowHandle::Win32(handle) = cc
            .window_handle()
            .map_err(|_| "无法获取窗口句柄，关闭窗口将退出程序")?
            .as_raw()
        else {
            return Err("当前窗口不支持 Windows 后台运行".into());
        };
        let signal = WakeSignal {
            ctx: cc.egui_ctx.clone(),
            hwnd: handle.hwnd.get() as usize,
            background: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(true)),
        };
        let (stop, receive) = mpsc::channel();
        let tick = signal.clone();
        let thread = std::thread::Builder::new()
            .name("tray-wakeup".into())
            .spawn(move || {
                while matches!(
                    receive.recv_timeout(Duration::from_secs(1)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    if tick.background.load(Ordering::Acquire) {
                        tick.wake();
                    }
                }
            })
            .map_err(|_| "无法启动后台唤醒任务，关闭窗口将退出程序")?;
        Ok(Self {
            signal,
            stop,
            thread: Some(thread),
        })
    }

    pub fn signal(&self) -> WakeSignal {
        self.signal.clone()
    }

    pub fn set_background(&self, background: bool) {
        self.signal.background.store(background, Ordering::Release);
    }
}

impl Drop for NativeWake {
    fn drop(&mut self) {
        self.signal.active.store(false, Ordering::Release);
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
