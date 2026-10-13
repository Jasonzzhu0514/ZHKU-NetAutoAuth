use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Shared application directory; credential and desktop settings stay separate.
pub fn app_data() -> Result<PathBuf, String> {
    data_root()
}

pub fn credentials() -> Result<PathBuf, String> {
    migrate(&app_data()?, "credentials.dat")
}

pub fn desktop_preferences() -> Result<PathBuf, String> {
    migrate(&app_data()?, "desktop.json")
}

fn data_root() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(|value| PathBuf::from(value).join("ZHKU-NetAutoAuth"))
        .ok_or_else(|| "无法确定 Windows 本地应用数据目录".into())
}

pub(crate) fn legacy_credentials(file: &Path) -> Option<PathBuf> {
    legacy_configuration(file).filter(|_| {
        file.file_name()
            .is_some_and(|name| name == "credentials.dat")
    })
}

fn legacy_configuration(file: &Path) -> Option<PathBuf> {
    let root = data_root().ok()?;
    let name = file.file_name()?;
    if name != "credentials.dat" && name != "desktop.json" {
        return None;
    }
    (file == root.join(name)).then(|| root.join("rust-gui").join(name))
}

pub(crate) fn sync_legacy_configuration(file: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(legacy) = legacy_configuration(file) {
        update_existing(&legacy, data)?;
    }
    Ok(())
}

fn update_existing(file: &Path, data: &[u8]) -> Result<(), String> {
    use io::Write;
    if !file.try_exists().map_err(|_| "无法检查旧版配置副本")? {
        return Ok(());
    }
    let parent = file.parent().ok_or("旧版配置目录无效")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "无法同步旧版配置副本，请检查文件权限")?;
    temporary
        .write_all(data)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| "无法同步旧版配置副本，请检查文件权限")?;
    temporary
        .persist(file)
        .map_err(|_| "无法替换旧版配置副本，请检查文件权限")?;
    Ok(())
}

fn migrate(root: &Path, name: &str) -> Result<PathBuf, String> {
    if let Ok(metadata) = fs::metadata(root)
        && !metadata.is_dir()
    {
        return Err("配置目录不是文件夹，原有文件已保留".into());
    }
    let legacy = root.join("rust-gui");
    let destination = root.join(name);
    if destination.try_exists().map_err(|_| "无法检查现有配置")? {
        // A newer configuration always takes precedence over the legacy file.
        return Ok(destination);
    }
    let source = legacy.join(name);
    let mut input = match fs::File::open(&source) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(destination),
        Err(_) => return Err("无法读取旧版配置，原文件已保留".into()),
    };
    fs::create_dir_all(root).map_err(|_| "无法创建配置目录，旧版配置已保留")?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(root).map_err(|_| "无法创建迁移文件，旧版配置已保留")?;
    let limit = if name == "credentials.dat" {
        64 * 1024
    } else {
        4096
    };
    let copied = io::copy(&mut input.by_ref().take(limit + 1), temporary.as_file_mut())
        .map_err(|_| "无法迁移配置，旧版配置已保留")?;
    if copied > limit {
        return Err("旧版配置过大，原文件已保留".into());
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| "无法迁移配置，旧版配置已保留")?;
    match temporary.persist_noclobber(&destination) {
        // Retain the legacy copy so rollback to an older executable keeps its account.
        Ok(_) => {}
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("无法保存迁移配置，旧版配置已保留".into()),
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_encrypted_bytes_and_settings_without_reencoding() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let legacy = root.join("rust-gui");
        fs::create_dir(&legacy).unwrap();
        let settings = br#"{"close_behavior":"tray"}"#;
        fs::write(legacy.join("credentials.dat"), b"encrypted-test-bytes").unwrap();
        fs::write(legacy.join("desktop.json"), settings).unwrap();
        migrate(root, "credentials.dat").unwrap();
        migrate(root, "desktop.json").unwrap();
        assert_eq!(
            fs::read(root.join("credentials.dat")).unwrap(),
            b"encrypted-test-bytes"
        );
        assert_eq!(fs::read(root.join("desktop.json")).unwrap(), settings);
        assert_eq!(
            fs::read(legacy.join("credentials.dat")).unwrap(),
            b"encrypted-test-bytes"
        );
        migrate(root, "credentials.dat").unwrap();
    }

    #[test]
    fn preserves_newer_files_and_unrelated_legacy_contents() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let legacy = root.join("rust-gui");
        fs::create_dir(&legacy).unwrap();
        fs::write(root.join("credentials.dat"), b"newer").unwrap();
        fs::write(legacy.join("credentials.dat"), b"older").unwrap();
        fs::write(legacy.join("unrelated.txt"), b"keep").unwrap();
        migrate(root, "credentials.dat").unwrap();
        assert_eq!(fs::read(root.join("credentials.dat")).unwrap(), b"newer");
        assert_eq!(fs::read(legacy.join("credentials.dat")).unwrap(), b"older");
        assert_eq!(fs::read(legacy.join("unrelated.txt")).unwrap(), b"keep");
    }

    #[test]
    fn invalid_root_leaves_existing_contents_intact() {
        let directory = tempfile::tempdir().unwrap();
        let blocked = directory.path().join("blocked");
        fs::write(&blocked, b"not-a-directory").unwrap();
        assert!(migrate(&blocked, "credentials.dat").is_err());
        assert_eq!(fs::read(blocked).unwrap(), b"not-a-directory");
    }

    #[test]
    fn broken_legacy_preferences_do_not_block_current_credentials() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("rust-gui").join("desktop.json")).unwrap();
        fs::write(root.join("credentials.dat"), b"valid-test-credentials").unwrap();
        assert!(migrate(root, "desktop.json").is_err());
        let credentials = migrate(root, "credentials.dat").unwrap();
        assert_eq!(fs::read(credentials).unwrap(), b"valid-test-credentials");
    }

    #[test]
    fn oversized_legacy_files_are_preserved_without_installing_partial_copies() {
        for (name, limit) in [("credentials.dat", 64 * 1024), ("desktop.json", 4096)] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path();
            fs::create_dir(root.join("rust-gui")).unwrap();
            let source = root.join("rust-gui").join(name);
            fs::write(&source, vec![b'x'; limit + 1]).unwrap();
            assert!(migrate(root, name).is_err());
            assert!(!root.join(name).exists());
            assert_eq!(fs::metadata(source).unwrap().len(), (limit + 1) as u64);
        }
    }

    #[test]
    fn updates_existing_rollback_copy_without_creating_new_legacy_files() {
        let directory = tempfile::tempdir().unwrap();
        let backup = directory.path().join("legacy.dat");
        update_existing(&backup, b"replacement-ciphertext").unwrap();
        assert!(!backup.exists());
        fs::write(&backup, b"old-ciphertext").unwrap();
        update_existing(&backup, b"replacement-ciphertext").unwrap();
        assert_eq!(fs::read(backup).unwrap(), b"replacement-ciphertext");
    }
}
