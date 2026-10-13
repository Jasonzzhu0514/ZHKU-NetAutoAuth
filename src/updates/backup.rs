use super::{download::hash_file, platform, registry};
use std::{fs, path::Path};

pub(super) struct Backup {
    hash: String,
    installed_version: Option<Option<String>>,
    uninstaller: bool,
}

impl Backup {
    pub fn create(target: &Path, directory: &Path, installed: bool) -> Result<Self, String> {
        fs::copy(target, directory.join("backup.exe")).map_err(|_| "无法备份原程序，更新已停止")?;
        let uninstaller = installed && target.with_file_name("Uninstall.exe").exists();
        if uninstaller {
            fs::copy(
                target.with_file_name("Uninstall.exe"),
                directory.join("backup-uninstall.exe"),
            )
            .map_err(|_| "无法备份卸载程序，更新已停止")?;
        }
        Ok(Self {
            hash: hash_file(&directory.join("backup.exe"))?,
            installed_version: installed.then(registry::version).transpose()?,
            uninstaller,
        })
    }

    pub fn restore(&self, target: &Path, directory: &Path) -> Result<(), String> {
        if hash_file(target).ok().as_ref() != Some(&self.hash) {
            restore_file(target, &directory.join("backup.exe"))?;
        }
        if let Some(version) = &self.installed_version {
            let uninstaller = target.with_file_name("Uninstall.exe");
            if self.uninstaller {
                restore_file(&uninstaller, &directory.join("backup-uninstall.exe"))?;
            } else if uninstaller.exists() {
                fs::remove_file(&uninstaller).map_err(|_| "无法恢复原卸载程序状态")?;
            }
            registry::restore_version(version.as_deref())?;
        }
        Ok(())
    }
}

fn restore_file(target: &Path, backup: &Path) -> Result<(), String> {
    if target.exists() {
        platform::replace(target, backup)
    } else {
        fs::rename(backup, target).map_err(|_| "旧版恢复失败，请手动恢复备份".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn rollback_restores_changed_or_missing_programs() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("client.exe");
        for missing in [false, true] {
            fs::write(&target, b"old application").unwrap();
            let backup = Backup::create(&target, directory.path(), false).unwrap();
            if missing {
                fs::remove_file(&target).unwrap();
            } else {
                fs::write(&target, b"bad update").unwrap();
            }
            backup.restore(&target, directory.path()).unwrap();
            assert_eq!(fs::read(&target).unwrap(), b"old application");
        }
    }
}
