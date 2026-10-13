use super::*;
use portal::{Credentials, Portal, State, Status};

impl Client {
    pub fn set_guard(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && !self.configured() {
            return Err("请先填写账号和密码".into());
        }
        self.guard = enabled;
        self.failures = 0;
        self.guard_pending = enabled && self.receiver.is_some();
        self.next_check = Instant::now();
        self.log(
            if enabled {
                "自动守护已开启"
            } else {
                "已停止自动守护；进行中的网络请求完成后不再重试"
            },
            false,
        );
        Ok(())
    }

    pub fn launch(&mut self, action: Action) -> Result<(), String> {
        if self.receiver.is_some() {
            return Err("请等待当前网络请求完成".into());
        }
        if matches!(action, Action::Login | Action::Guard) && !self.configured() {
            return Err("请先填写账号和密码".into());
        }
        if matches!(action, Action::Check) {
            self.log("正在刷新网络状态…", false);
        }
        if matches!(action, Action::Logout) {
            self.guard = false;
            self.guard_pending = false;
            if self.settings.auto_login {
                self.settings.auto_login = false;
                if let Err(error) =
                    storage::path().and_then(|path| storage::save(&path, &self.settings))
                {
                    self.log(error, true);
                }
            }
            self.log("正在注销，自动守护已停止", false);
        }
        let credentials = if matches!(action, Action::Check | Action::Logout) {
            None
        } else {
            Some(Credentials {
                username: self.settings.username.trim().into(),
                password: self.settings.password.clone(),
            })
        };
        let (sender, receiver) = mpsc::channel();
        let (trace_sender, traces) = mpsc::channel();
        self.receiver = Some(receiver);
        self.trace_receiver = Some(traces);
        self.mark_changed();
        std::thread::spawn(move || {
            let event = execute(action, credentials, trace_sender);
            let _ = sender.send(event);
        });
        Ok(())
    }

    pub fn tick(&mut self) {
        // A completed event guarantees that all traces from that worker were sent.
        // Read it before draining traces so a new job cannot replace undrained logs.
        let received = self.receiver.as_ref().map(|receiver| receiver.try_recv());
        if let Some(receiver) = &self.trace_receiver {
            let traces: Vec<_> = receiver.try_iter().collect();
            for trace in traces {
                self.logs.push(LogEntry {
                    id: self.next_log_id,
                    elapsed_ms: trace
                        .at
                        .saturating_duration_since(self.started_at)
                        .as_millis() as u64,
                    level: if trace.failed { "ERROR" } else { "INFO" },
                    message: trace.message,
                });
                self.next_log_id += 1;
                self.mark_changed();
            }
        }
        match received {
            Some(Ok(event)) => {
                self.receiver = None;
                self.status = event.status;
                self.failures = if event.failed {
                    (self.failures + 1).min(5)
                } else {
                    0
                };
                let delay = self.preferences.guard_delay_seconds(self.failures);
                self.next_check = Instant::now()
                    + if self.guard_pending {
                        Duration::ZERO
                    } else {
                        Duration::from_secs(delay)
                    };
                self.guard_pending = false;
                self.log(event.message, event.failed);
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.receiver = None;
                self.guard = false;
                self.guard_pending = false;
                self.log("网络任务异常结束，自动守护已停止", true);
            }
            _ => {}
        }
        if self.guard && self.receiver.is_none() && Instant::now() >= self.next_check {
            let _ = self.launch(Action::Guard);
        }
    }
}

fn execute(
    action: Action,
    credentials: Option<Credentials>,
    trace: mpsc::Sender<portal::Trace>,
) -> NetworkEvent {
    let portal = match Portal::new().map(|portal| portal.with_trace(trace)) {
        Ok(portal) => portal,
        Err(error) => {
            return NetworkEvent {
                status: Status::unknown("未检测到", &error),
                message: error,
                failed: true,
            };
        }
    };
    if matches!(action, Action::Logout) {
        return match portal.logout() {
            Ok(status) => NetworkEvent {
                status,
                message: "已确认离线，自动守护保持停止".into(),
                failed: false,
            },
            Err(error) => NetworkEvent {
                status: portal.status(),
                message: error,
                failed: true,
            },
        };
    }
    let status = portal.status();
    if !matches!(action, Action::Check) && status.state == State::Offline {
        return match portal.login(credentials.as_ref().expect("login credentials")) {
            Ok(status) => NetworkEvent {
                status,
                message: "认证成功，已确认在线".into(),
                failed: false,
            },
            Err(error) => NetworkEvent {
                status: portal.status(),
                message: error,
                failed: true,
            },
        };
    }
    let failed = status.state == State::Unknown;
    let message = match status.state {
        State::Online => "已通过认证".into(),
        State::Offline => "已连接校园网，尚未认证".into(),
        State::Unknown => status.detail.clone(),
    };
    NetworkEvent {
        status,
        message,
        failed,
    }
}
