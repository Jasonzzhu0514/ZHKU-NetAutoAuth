use super::{
    backup::Backup,
    download::{Prepared, hash_file},
    platform,
    releases::{InstallMode, Release},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
struct Plan {
    target: PathBuf,
    parent_pid: u32,
    version: String,
    sha256: String,
    installed: bool,
    resume_guard: bool,
}

pub(super) fn launch(
    prepared: Prepared,
    release: &Release,
    mode: InstallMode,
    resume_guard: bool,
) -> Result<(), String> {
    let helper = prepared.directory.path().join("helper.exe");
    let current = std::env::current_exe().map_err(|_| "无法定位当前程序")?;
    fs::copy(&current, &helper).map_err(|_| "无法创建更新助手")?;
    let plan = Plan {
        target: prepared.target,
        parent_pid: std::process::id(),
        version: release.version.to_string(),
        sha256: prepared.hash,
        installed: mode == InstallMode::Installed,
        resume_guard,
    };
    let manifest = prepared.directory.path().join("update-plan.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&plan).map_err(|_| "无法生成更新计划")?,
    )
    .map_err(|_| "无法保存更新计划")?;
    platform::spawn(Command::new(helper).arg("--apply-update").arg(&manifest))
        .map_err(|_| "无法启动更新助手，原程序保持运行")?;
    // Ownership passes to the helper only after it was started successfully.
    let _ = prepared.directory.keep();
    Ok(())
}

fn validated_plan(manifest: &Path, helper: &Path) -> Result<(Plan, PathBuf), String> {
    let directory = helper
        .parent()
        .ok_or("更新助手路径无效")?
        .canonicalize()
        .map_err(|_| "更新目录无效")?;
    if !directory
        .file_name()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.starts_with(".zhku-update-"))
    {
        return Err("更新助手目录无效".into());
    }
    if manifest.canonicalize().map_err(|_| "更新计划不可读")? != directory.join("update-plan.json")
    {
        return Err("更新计划路径无效".into());
    }
    let data = fs::read(manifest).map_err(|_| "无法读取更新计划")?;
    if data.len() > 16 * 1024 {
        return Err("更新计划过大".into());
    }
    let plan: Plan = serde_json::from_slice(&data).map_err(|_| "更新计划无效")?;
    let target = plan.target.canonicalize().map_err(|_| "原程序不存在")?;
    if target.parent() != directory.parent()
        || !target
            .extension()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("exe"))
    {
        return Err("更新目标不在当前程序目录".into());
    }
    if semver::Version::parse(&plan.version).is_err() || plan.sha256.len() != 64 {
        return Err("更新计划版本或校验值无效".into());
    }
    if hash_file(&directory.join("payload.exe"))? != plan.sha256 {
        return Err("更新包校验失败，未修改原程序".into());
    }
    Ok((Plan { target, ..plan }, directory))
}

fn wait_child(child: &mut std::process::Child, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err("安装包执行失败".into())
                };
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("安装更新超时或异常结束".into());
            }
        }
    }
}

fn start(plan: &Plan, marker: &Path, failure: Option<&str>) -> Result<std::process::Child, String> {
    let mut command = Command::new(&plan.target);
    if let Some(reason) = failure {
        command.arg("--update-failed").arg(reason);
    } else {
        command.arg("--update-confirm").arg(marker);
    }
    if plan.resume_guard {
        command.arg("--resume-guard");
    }
    platform::spawn(&mut command).map_err(|_| "无法重新启动程序，请手动打开".into())
}

fn version_output(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = platform::spawn(&mut command).map_err(|_| "新版程序无法运行")?;
    wait_child(&mut child, Duration::from_secs(10))?;
    let mut text = String::new();
    child
        .stdout
        .take()
        .ok_or("无法获取新版程序版本")?
        .take(4096)
        .read_to_string(&mut text)
        .map_err(|_| "新版程序版本输出无效")?;
    Ok(text.trim().into())
}

fn apply(plan: &Plan, directory: &Path) -> Result<(), String> {
    let payload = directory.join("payload.exe");
    if plan.installed {
        if platform::mode(&plan.target) != InstallMode::Installed {
            return Err("安装位置已变化，请手动更新".into());
        }
        let mut child =
            platform::spawn(Command::new(payload).arg("/S")).map_err(|_| "无法启动安装包")?;
        wait_child(&mut child, Duration::from_secs(180))?;
        if platform::mode(&plan.target) != InstallMode::Installed
            || super::registry::version()?.as_deref() != Some(plan.version.as_str())
        {
            return Err("安装结果与更新计划不符".into());
        }
    } else {
        platform::replace(&plan.target, &payload)?;
    }
    if version_output(&plan.target)? != format!("ZHKU-NetAutoAuth {}", plan.version) {
        return Err("新版程序版本验证失败".into());
    }
    let marker = directory.join("started");
    let mut child = start(plan, &marker, None)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if fs::read(&marker).ok().as_deref() == Some(b"ready") {
            return Ok(());
        }
        if Instant::now() >= deadline || child.try_wait().ok().flatten().is_some() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("新版程序启动失败，正在恢复旧版本".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(crate) fn run_helper(manifest: &Path) -> Result<(), String> {
    let helper = std::env::current_exe().map_err(|_| "更新助手路径无效")?;
    let (plan, directory) = validated_plan(manifest, &helper)?;
    platform::wait_parent(plan.parent_pid)?;
    let backup = Backup::create(&plan.target, &directory, plan.installed)?;
    if let Err(reason) = apply(&plan, &directory) {
        backup.restore(&plan.target, &directory)?;
        // Internal errors contain no credentials, URLs or personal file paths.
        start(&plan, &directory.join("started"), Some(&reason))?;
    }
    // Do not remove the running helper. A later startup cleans our completed directory.
    let _ = fs::remove_file(directory.join("payload.exe"));
    let _ = fs::remove_file(directory.join("backup.exe"));
    let _ = fs::remove_file(directory.join("backup-uninstall.exe"));
    let _ = fs::remove_file(manifest);
    let _ = fs::write(directory.join("done"), b"ZHKU-NetAutoAuth updater complete");
    Ok(())
}

pub(crate) fn resume_requested() -> bool {
    std::env::args_os().any(|arg| arg == "--resume-guard")
}
pub(crate) fn failure_reported() -> bool {
    std::env::args_os().any(|arg| arg == "--update-failed")
}

pub(crate) fn failure_reason() -> Option<String> {
    reported_reason(&std::env::args_os().collect::<Vec<_>>()).map(str::to_owned)
}

fn reported_reason(args: &[std::ffi::OsString]) -> Option<&str> {
    let index = args.iter().position(|arg| arg == "--update-failed")?;
    args.get(index + 1)?
        .to_str()
        .filter(|reason| !reason.is_empty() && !reason.starts_with("--") && reason.len() <= 1024)
}

pub(crate) fn report_error(message: &str) {
    platform::report_error(message);
}

pub(crate) fn confirm_startup() {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--update-confirm")
        && let Some(path) = args.get(index + 1)
    {
        let marker = Path::new(path);
        if let Ok(executable) = std::env::current_exe()
            && valid_startup_marker(marker, &executable)
        {
            let _ = fs::write(marker, b"ready");
        }
    }
}

fn valid_startup_marker(marker: &Path, executable: &Path) -> bool {
    let (Some(parent), Ok(executable)) = (marker.parent(), executable.canonicalize()) else {
        return false;
    };
    let Ok(directory) = parent.canonicalize() else {
        return false;
    };
    marker.file_name().is_some_and(|v| v == "started")
        && directory.parent() == executable.parent()
        && directory
            .file_name()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.starts_with(".zhku-update-"))
}

pub(super) fn cleanup_completed(target: &Path) {
    let Some(parent) = target.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".zhku-update-")
            && entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
            && path
                .canonicalize()
                .is_ok_and(|resolved| resolved.parent() == parent.canonicalize().ok().as_deref())
            && fs::read(path.join("done")).ok().as_deref()
                == Some(b"ZHKU-NetAutoAuth updater complete")
        {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rollback_reason_does_not_consume_flags_or_unbounded_text() {
        let args =
            |reason: &str| vec!["client.exe".into(), "--update-failed".into(), reason.into()];
        assert_eq!(
            reported_reason(&args("新版程序版本验证失败")),
            Some("新版程序版本验证失败")
        );
        assert_eq!(reported_reason(&args("--resume-guard")), None);
        assert_eq!(reported_reason(&args(&"x".repeat(1025))), None);
        assert_eq!(reported_reason(&["--update-failed".into()]), None);
    }
    #[test]
    fn startup_confirmation_accepts_normal_paths_only_in_owned_program_directory() {
        let root = tempfile::tempdir().unwrap();
        let stage = root.path().join(".zhku-update-test");
        fs::create_dir(&stage).unwrap();
        let target = root.path().join("client.exe");
        fs::write(&target, b"program").unwrap();
        assert!(valid_startup_marker(&stage.join("started"), &target));
        assert!(!valid_startup_marker(&stage.join("other"), &target));
        assert!(!valid_startup_marker(&root.path().join("started"), &target));
        assert!(!valid_startup_marker(
            &stage.join("started"),
            &stage.join("helper.exe")
        ));
    }
    #[test]
    fn plans_outside_helper_directory_and_modified_payloads_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let stage = tempfile::Builder::new()
            .prefix(".zhku-update-")
            .tempdir_in(root.path())
            .unwrap();
        let target = root.path().join("client.exe");
        fs::write(&target, b"old").unwrap();
        let payload = stage.path().join("payload.exe");
        fs::write(&payload, b"new").unwrap();
        let plan = Plan {
            target,
            parent_pid: 0,
            version: "2.2.0".into(),
            sha256: hash_file(&payload).unwrap(),
            installed: false,
            resume_guard: false,
        };
        let manifest = stage.path().join("update-plan.json");
        fs::write(&manifest, serde_json::to_vec(&plan).unwrap()).unwrap();
        let helper = stage.path().join("helper.exe");
        assert!(validated_plan(&manifest, &helper).is_ok());
        fs::write(&payload, b"tampered").unwrap();
        assert!(validated_plan(&manifest, &helper).is_err());
        assert!(validated_plan(&root.path().join("update-plan.json"), &helper).is_err());
    }
    #[test]
    fn cleanup_leaves_unowned_and_incomplete_directories_intact() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("client.exe");
        for name in [
            "ordinary",
            ".zhku-update-incomplete",
            ".zhku-update-complete",
        ] {
            let folder = root.path().join(name);
            fs::create_dir(&folder).unwrap();
            if name.ends_with("complete") && !name.ends_with("incomplete") {
                fs::write(folder.join("done"), b"ZHKU-NetAutoAuth updater complete").unwrap();
            }
        }
        cleanup_completed(&app);
        assert!(root.path().join("ordinary").exists());
        assert!(root.path().join(".zhku-update-incomplete").exists());
        assert!(!root.path().join(".zhku-update-complete").exists());
    }
}
