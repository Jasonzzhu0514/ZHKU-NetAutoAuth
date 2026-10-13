use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
const MAX_FILE_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub struct Settings {
    pub username: String,
    pub password: Zeroizing<String>,
    pub remember: bool,
    pub auto_login: bool,
}

impl Settings {
    fn validate(&self) -> Result<(), String> {
        if self.username.len() > 256 || self.password.len() > 1024 {
            return Err("账号或密码过长，凭据未保存".into());
        }
        Ok(())
    }
    pub fn normalize(&mut self) {
        if !self.remember {
            self.auto_login = false;
            use zeroize::Zeroize;
            self.password.zeroize();
        }
        if self.username.trim().is_empty() || self.password.is_empty() {
            self.auto_login = false;
        }
    }
}

pub fn path() -> Result<PathBuf, String> {
    crate::paths::credentials().map_err(|error| format!("{error}，凭据未保存"))
}

#[cfg(windows)]
fn crypt(bytes: &[u8], decrypt: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        },
    };
    if bytes.is_empty() {
        return Err("凭据数据为空，请重新配置".into());
    }
    let mut input = Zeroizing::new(bytes.to_vec());
    let blob = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).map_err(|_| "凭据数据过大")?,
        pbData: input.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI buffers are owned by Windows and must be released with LocalFree.
    unsafe {
        let success = if decrypt {
            CryptUnprotectData(
                &blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if success == 0 {
            return Err("Windows 凭据加解密失败，请使用保存凭据的 Windows 用户重新配置".into());
        }
        if output.pbData.is_null() || output.cbData == 0 {
            if !output.pbData.is_null() {
                LocalFree(output.pbData.cast());
            }
            return Err("Windows 返回空凭据数据，请重新配置".into());
        }
        let result = Zeroizing::new(
            std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec(),
        );
        if decrypt {
            use zeroize::Zeroize;
            std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize).zeroize();
        }
        LocalFree(output.pbData.cast());
        Ok(result)
    }
}

#[cfg(not(windows))]
fn crypt(_: &[u8], _: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    Err("此客户端的凭据保存仅支持 Windows".into())
}

pub fn load(file: &Path) -> Result<Settings, String> {
    let source = match std::fs::File::open(file) {
        Ok(source) => source,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(_) => return Err("无法读取保存的凭据".into()),
    };
    let mut encrypted = Vec::new();
    source
        .take(MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut encrypted)
        .map_err(|_| "无法读取保存的凭据")?;
    if encrypted.len() > MAX_FILE_BYTES {
        return Err("凭据文件异常，请删除后重新配置".into());
    }
    let plain = crypt(&encrypted, true)?;
    let mut value: serde_json::Value =
        serde_json::from_slice(&plain).map_err(|_| "凭据文件损坏，请重新配置")?;
    let obj = value.as_object_mut().ok_or("凭据格式异常")?;
    let mut settings = Settings {
        username: obj
            .remove("username")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        password: Zeroizing::new(match obj.remove("password") {
            Some(serde_json::Value::String(s)) => s,
            _ => String::new(),
        }),
        remember: obj
            .get("remember")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        auto_login: obj
            .get("auto_login")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    settings.normalize();
    settings.validate()?;
    Ok(settings)
}

pub fn save(file: &Path, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    let password = if settings.remember {
        settings.password.as_str()
    } else {
        ""
    };
    let mut data = serde_json::json!({
        "username": settings.username.trim(), "password": password,
        "remember": settings.remember,
        "auto_login": settings.auto_login && settings.remember && !password.is_empty() && !settings.username.trim().is_empty(),
    });
    let plain = Zeroizing::new(serde_json::to_vec(&data).map_err(|_| "无法生成凭据配置")?);
    if let Some(serde_json::Value::String(password)) = data.get_mut("password") {
        use zeroize::Zeroize;
        password.zeroize();
    }
    let encrypted = crypt(&plain, false)?;
    let parent = file.parent().ok_or("凭据保存目录无效")?;
    std::fs::create_dir_all(parent).map_err(|_| "无法创建凭据目录")?;
    // Both the temporary and final files contain ciphertext only. Persist replaces atomically.
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法创建加密凭据文件")?;
    temp.write_all(&encrypted)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|_| "无法写入加密凭据")?;
    temp.persist(file).map_err(|_| "无法替换加密凭据文件")?;
    crate::paths::sync_legacy_configuration(file, &encrypted)?;
    Ok(())
}

pub fn delete(file: &Path) -> Result<(), String> {
    let legacy = crate::paths::legacy_credentials(file);
    delete_with_legacy(file, legacy.as_deref())
}

fn delete_with_legacy(file: &Path, legacy: Option<&Path>) -> Result<(), String> {
    // Remove legacy credentials first so a failed deletion cannot resurrect them.
    for path in legacy.into_iter().chain(std::iter::once(file)) {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法删除保存的凭据，请检查文件权限".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleting_credentials_also_removes_legacy_copy_without_touching_settings() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("credentials.dat");
        let legacy = dir.path().join("old-credentials.dat");
        let settings = dir.path().join("desktop.json");
        std::fs::write(&current, b"test-encrypted-current").unwrap();
        std::fs::write(&legacy, b"test-encrypted-legacy").unwrap();
        std::fs::write(&settings, b"keep").unwrap();
        delete_with_legacy(&current, Some(&legacy)).unwrap();
        assert!(!current.exists());
        assert!(!legacy.exists());
        assert_eq!(std::fs::read(settings).unwrap(), b"keep");
        delete_with_legacy(&current, Some(&legacy)).unwrap();
    }
    #[test]
    fn invalid_or_oversized_credentials_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("credentials.dat");
        for content in [
            Vec::new(),
            b"not encrypted".to_vec(),
            vec![0; MAX_FILE_BYTES + 1],
        ] {
            std::fs::write(&file, content).unwrap();
            assert!(load(&file).is_err());
        }
        let oversized = Settings {
            username: "x".repeat(257),
            ..Default::default()
        };
        assert!(save(&file, &oversized).is_err());
    }
    #[test]
    fn forgetting_password_disables_auto_login() {
        let mut settings = Settings {
            username: "example".into(),
            password: Zeroizing::new("example-password".into()),
            remember: false,
            auto_login: true,
        };
        settings.normalize();
        assert!(settings.password.is_empty());
        assert!(!settings.auto_login);
    }
    #[test]
    #[cfg(windows)]
    fn encrypted_storage_roundtrip_replacement_and_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("credentials.dat");
        let mut settings = Settings {
            username: "example".into(),
            password: Zeroizing::new("example-password".into()),
            remember: true,
            auto_login: true,
        };
        save(&file, &settings).unwrap();
        assert!(
            !std::fs::read(&file)
                .unwrap()
                .windows(b"example-password".len())
                .any(|w| w == b"example-password")
        );
        let loaded = load(&file).unwrap();
        assert_eq!(*loaded.password, "example-password");
        assert!(loaded.auto_login);
        settings.remember = false;
        save(&file, &settings).unwrap();
        let loaded = load(&file).unwrap();
        assert!(loaded.password.is_empty());
        assert!(!loaded.auto_login);
        assert_eq!(loaded.username, "example");
        delete(&file).unwrap();
        assert!(!file.exists());
        delete(&file).unwrap();
    }
}
