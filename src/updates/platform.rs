use super::releases::InstallMode;
use std::path::Path;

#[cfg(windows)]
pub(super) fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(Some(0)).collect()
}

pub(super) fn mode(target: &Path) -> InstallMode {
    if let Some(directory) = super::registry::install_dir()
        && let (Ok(registered), Ok(actual)) = (
            directory.join("ZHKU-NetAutoAuth.exe").canonicalize(),
            target.canonicalize(),
        )
        && registered == actual
    {
        return InstallMode::Installed;
    }
    InstallMode::Portable
}

pub(super) fn wait_parent(pid: u32) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, GetLastError, WAIT_OBJECT_0},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return if GetLastError() == ERROR_INVALID_PARAMETER {
                Ok(()) // The original process has already exited.
            } else {
                Err("无法确认旧程序已退出，已停止更新".into())
            };
        }
        let result = WaitForSingleObject(handle, 60_000);
        CloseHandle(handle);
        if result != WAIT_OBJECT_0 {
            return Err("旧程序尚未退出，已停止更新".into());
        }
    }
    Ok(())
}

pub(super) fn replace(target: &Path, replacement: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW,
        };
        let target = wide(target.as_os_str());
        let replacement = wide(replacement.as_os_str());
        let success = unsafe {
            ReplaceFileW(
                target.as_ptr(),
                replacement.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_IGNORE_MERGE_ERRORS,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if success == 0 {
            return Err("程序文件无法替换，请关闭其他实例或手动更新".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (target, replacement);
        Err("自动安装更新目前仅支持 Windows".into())
    }
}

pub(super) fn spawn(command: &mut std::process::Command) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    command.spawn()
}

pub(super) fn report_error(message: &str) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let text = wide(std::ffi::OsStr::new(message));
        let title = wide(std::ffi::OsStr::new("ZHKU-NetAutoAuth 更新未完成"));
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
    #[cfg(not(windows))]
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn atomic_replacement_can_restore_original_contents() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("client.exe");
        let next = directory.path().join("new.exe");
        let backup = directory.path().join("backup.exe");
        std::fs::write(&target, b"old application").unwrap();
        std::fs::copy(&target, &backup).unwrap();
        std::fs::write(&next, b"new application").unwrap();
        super::replace(&target, &next).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new application");
        super::replace(&target, &backup).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old application");
    }
}
