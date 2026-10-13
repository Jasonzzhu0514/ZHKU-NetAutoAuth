use std::path::PathBuf;

const INSTALL_KEY: &str = "Software\\ZHKU-NetAutoAuth";
const UNINSTALL_KEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ZHKU-NetAutoAuth";

#[cfg(windows)]
fn read(key: &str, name: &str) -> Result<Option<String>, String> {
    use windows_sys::Win32::{
        Foundation::ERROR_FILE_NOT_FOUND,
        System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW},
    };
    let key = super::platform::wide(std::ffi::OsStr::new(key));
    let name = super::platform::wide(std::ffi::OsStr::new(name));
    let mut buffer = [0u16; 32768];
    let mut size = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    match result {
        0 => {
            let length = buffer.iter().position(|n| *n == 0).unwrap_or(buffer.len());
            String::from_utf16(&buffer[..length])
                .map(Some)
                .map_err(|_| "安装信息格式无效".into())
        }
        ERROR_FILE_NOT_FOUND => Ok(None),
        _ => Err("无法读取安装信息，已停止更新".into()),
    }
}

#[cfg(not(windows))]
fn read(_key: &str, _name: &str) -> Result<Option<String>, String> {
    Ok(None)
}

pub(super) fn install_dir() -> Option<PathBuf> {
    read(INSTALL_KEY, "InstallDir")
        .ok()
        .flatten()
        .map(PathBuf::from)
}

pub(super) fn version() -> Result<Option<String>, String> {
    read(UNINSTALL_KEY, "DisplayVersion")
}

pub(super) fn restore_version(value: Option<&str>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW},
        };
        let key = super::platform::wide(std::ffi::OsStr::new(UNINSTALL_KEY));
        let name = super::platform::wide(std::ffi::OsStr::new("DisplayVersion"));
        let result = unsafe {
            if let Some(value) = value {
                let data = super::platform::wide(std::ffi::OsStr::new(value));
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    key.as_ptr(),
                    name.as_ptr(),
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            } else {
                RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr())
            }
        };
        if result != 0 && !(value.is_none() && result == ERROR_FILE_NOT_FOUND) {
            return Err("旧程序已恢复，但无法恢复系统中的版本信息".into());
        }
    }
    #[cfg(not(windows))]
    let _ = value;
    Ok(())
}
