mod config;
mod connection;
#[cfg(test)]
mod connection_tests;
mod model;
mod probe;
mod route;
#[cfg(test)]
mod tests;

pub use model::Snapshot;
use model::{Entry, MAX_TARGETS, Target, TestResult, defaults, normalize};
use std::{path::PathBuf, sync::mpsc};

pub struct Service {
    path: Option<PathBuf>,
    entries: Vec<Entry>,
    receiver: Option<mpsc::Receiver<(u64, TestResult)>>,
    pending: Vec<u64>,
    revision: u64,
    warning: Option<String>,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            path: None,
            entries: entries(defaults()),
            receiver: None,
            pending: Vec::new(),
            revision: 0,
            warning: None,
        }
    }
}

fn entries(targets: Vec<Target>) -> Vec<Entry> {
    targets
        .into_iter()
        .map(|target| Entry {
            target,
            result: None,
        })
        .collect()
}

impl Service {
    pub fn initialize(&mut self) {
        match crate::paths::app_data() {
            Ok(root) => {
                let path = root.join("network-tests.json");
                match config::load(&path) {
                    Ok(targets) => self.entries = entries(targets),
                    Err(error) => {
                        self.warning = Some(format!(
                            "{error}，暂使用默认网站。新增或删除时会保存当前列表。"
                        ))
                    }
                }
                self.path = Some(path);
            }
            Err(error) => self.warning = Some(error),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            entries: self.entries.clone(),
            pending: self.pending.clone(),
            busy: self.receiver.is_some(),
            warning: self.warning.clone(),
        }
    }

    fn editable(&self) -> Result<(), String> {
        if self.receiver.is_some() {
            return Err("请等待当前网站测试完成".into());
        }
        Ok(())
    }

    fn save(&self, targets: &[Target]) -> Result<(), String> {
        config::save(
            self.path.as_ref().ok_or("无法确定网站列表保存位置")?,
            targets,
        )
    }

    pub fn add(&mut self, name: &str, url: &str) -> Result<(), String> {
        self.editable()?;
        if self.entries.len() >= MAX_TARGETS {
            return Err("最多保存 32 个网站".into());
        }
        let (name, url) = normalize(name, url)?;
        let mut targets: Vec<_> = self
            .entries
            .iter()
            .map(|entry| entry.target.clone())
            .collect();
        if targets.iter().any(|target| {
            normalize(&target.name, &target.url).is_ok_and(|(_, existing)| existing == url)
        }) {
            return Err("此网址已在列表中".into());
        }
        let id = targets
            .iter()
            .map(|target| target.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("网站标识已耗尽")?;
        targets.push(Target { id, name, url });
        self.save(&targets)?;
        self.entries.push(Entry {
            target: targets.pop().expect("new target"),
            result: None,
        });
        self.warning = None;
        self.revision += 1;
        Ok(())
    }

    pub fn remove(&mut self, id: u64) -> Result<(), String> {
        self.editable()?;
        if !self.entries.iter().any(|entry| entry.target.id == id) {
            return Err("网站已不存在".into());
        }
        let targets: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.target.id != id)
            .map(|entry| entry.target.clone())
            .collect();
        self.save(&targets)?;
        self.entries.retain(|entry| entry.target.id != id);
        self.warning = None;
        self.revision += 1;
        Ok(())
    }

    pub fn start(&mut self, id: Option<u64>) -> Result<(), String> {
        self.editable()?;
        let targets: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| id.is_none_or(|id| entry.target.id == id))
            .map(|entry| entry.target.clone())
            .collect();
        if targets.is_empty() {
            return Err("请先添加要测试的网站".into());
        }
        self.pending = targets.iter().map(|target| target.id).collect();
        for entry in &mut self.entries {
            if self.pending.contains(&entry.target.id) {
                entry.result = None;
            }
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.revision += 1;
        std::thread::spawn(move || {
            // Bound concurrency independently of the authentication worker.
            for group in targets.chunks(3) {
                std::thread::scope(|scope| {
                    for target in group {
                        let sender = &sender;
                        scope.spawn(move || {
                            let _ = sender.send((target.id, probe::run(target)));
                        });
                    }
                });
            }
        });
        Ok(())
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut finished = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok((id, result)) => {
                        if let Some(entry) =
                            self.entries.iter_mut().find(|entry| entry.target.id == id)
                        {
                            entry.result = Some(result);
                        }
                        self.pending.retain(|pending| *pending != id);
                        changed = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }
        if finished {
            self.receiver = None;
            if !self.pending.is_empty() {
                self.warning = Some("部分网站测试异常结束，请重试".into());
            }
            self.pending.clear();
            changed = true;
        }
        if changed {
            self.revision += 1;
        }
        changed
    }
}
