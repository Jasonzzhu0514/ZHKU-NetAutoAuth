use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloseBehavior {
    #[default]
    Tray,
    Exit,
}

pub(super) struct Preferences {
    pub close: CloseBehavior,
    pub notice_acknowledged: bool,
    pub auto_check_updates: bool,
    pub guard_interval_seconds: u64,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            close: CloseBehavior::Tray,
            notice_acknowledged: false,
            auto_check_updates: true,
            guard_interval_seconds: 30,
        }
    }
}

impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if !(30..=3600).contains(&self.guard_interval_seconds) {
            return Err("守护检查间隔必须为 30～3600 秒".into());
        }
        Ok(())
    }

    pub fn guard_delay_seconds(&self, failures: u32) -> u64 {
        let base = self.guard_interval_seconds;
        base.saturating_mul(2u64.pow(failures.min(5)))
            .min(300.max(base))
    }
    pub fn load(file: &Path) -> Result<Self, String> {
        let source = match std::fs::File::open(file) {
            Ok(source) => source,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(_) => return Err("无法读取窗口行为设置，已使用默认设置".into()),
        };
        let mut bytes = Vec::new();
        source
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| "无法读取窗口行为设置，已使用默认设置")?;
        if bytes.len() > 4096 {
            return Err("窗口行为设置文件异常，已使用默认设置".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "窗口行为设置文件损坏，已使用默认设置")?;
        if !value.is_object() {
            return Err("窗口行为设置格式异常，已使用默认设置".into());
        }
        let settings = Self {
            close: if value["close_behavior"].as_str() == Some("exit") {
                CloseBehavior::Exit
            } else {
                CloseBehavior::Tray
            },
            notice_acknowledged: value["notice_acknowledged"].as_bool().unwrap_or(false),
            auto_check_updates: value["auto_check_updates"].as_bool().unwrap_or(true),
            guard_interval_seconds: match value.get("guard_interval_seconds") {
                None => 30,
                Some(interval) => interval.as_u64().ok_or("守护检查间隔必须是整数秒数")?,
            },
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, file: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = file.parent().ok_or("窗口行为设置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|_| "无法创建窗口行为设置目录")?;
        // This file contains no credentials or device identifiers.
        let data = serde_json::json!({
            "close_behavior": if self.close == CloseBehavior::Tray { "tray" } else { "exit" },
            "notice_acknowledged": self.notice_acknowledged,
            "auto_check_updates": self.auto_check_updates,
            "guard_interval_seconds": self.guard_interval_seconds,
        });
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法创建窗口行为设置文件")?;
        temporary
            .write_all(data.to_string().as_bytes())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| "无法保存窗口行为设置")?;
        temporary
            .persist(file)
            .map_err(|_| "无法替换窗口行为设置")?;
        crate::paths::sync_legacy_configuration(file, data.to_string().as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_roundtrip_and_defaults_do_not_store_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.json");
        let mut settings = Preferences::load(&file).unwrap();
        assert_eq!(settings.close, CloseBehavior::Tray);
        assert!(!settings.notice_acknowledged);
        assert!(settings.auto_check_updates);
        settings.close = CloseBehavior::Exit;
        settings.notice_acknowledged = true;
        settings.auto_check_updates = false;
        settings.guard_interval_seconds = 90;
        settings.save(&file).unwrap();
        let loaded = Preferences::load(&file).unwrap();
        assert_eq!(loaded.close, CloseBehavior::Exit);
        assert!(loaded.notice_acknowledged);
        assert!(!loaded.auto_check_updates);
        assert_eq!(loaded.guard_interval_seconds, 90);
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 4);
    }

    #[test]
    fn legacy_preferences_keep_existing_choices_and_enable_update_checks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.json");
        std::fs::write(
            &file,
            br#"{"close_behavior":"exit","notice_acknowledged":true}"#,
        )
        .unwrap();
        let loaded = Preferences::load(&file).unwrap();
        assert_eq!(loaded.close, CloseBehavior::Exit);
        assert!(loaded.notice_acknowledged);
        assert!(loaded.auto_check_updates);
        assert_eq!(loaded.guard_interval_seconds, 30);
    }

    #[test]
    fn corrupt_preferences_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.json");
        for data in [b"[]".to_vec(), b"broken".to_vec(), vec![0; 4097]] {
            std::fs::write(&file, data).unwrap();
            assert!(Preferences::load(&file).is_err());
        }
    }

    #[test]
    fn invalid_intervals_are_rejected_without_overwriting_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.json");
        let mut settings = Preferences::default();
        settings.save(&file).unwrap();
        for interval in [0, 29, 3601, u64::MAX] {
            settings.guard_interval_seconds = interval;
            assert!(settings.save(&file).is_err());
            assert_eq!(Preferences::load(&file).unwrap().guard_interval_seconds, 30);
        }
        for value in [
            serde_json::json!(29),
            serde_json::json!(3601),
            serde_json::json!(1.5),
            serde_json::json!("60"),
        ] {
            std::fs::write(
                &file,
                serde_json::json!({"guard_interval_seconds":value}).to_string(),
            )
            .unwrap();
            assert!(Preferences::load(&file).is_err());
        }
    }

    #[test]
    fn backoff_preserves_custom_period_and_does_not_overflow() {
        let mut settings = Preferences {
            guard_interval_seconds: 90,
            ..Preferences::default()
        };
        assert_eq!(settings.guard_delay_seconds(0), 90);
        assert_eq!(settings.guard_delay_seconds(1), 180);
        assert_eq!(settings.guard_delay_seconds(2), 300);
        settings.guard_interval_seconds = 600;
        assert_eq!(settings.guard_delay_seconds(0), 600);
        assert_eq!(settings.guard_delay_seconds(u32::MAX), 600);
    }
}
