use super::releases::{Release, client};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct Prepared {
    pub directory: tempfile::TempDir,
    pub target: PathBuf,
    pub hash: String,
}

pub(super) fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|_| "无法读取更新文件")?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|_| "无法读取更新文件")?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn verify_pe(path: &Path) -> Result<(), String> {
    use std::io::{Seek, SeekFrom};
    let mut file = File::open(path).map_err(|_| "无法读取更新包")?;
    let mut header = [0; 64];
    file.read_exact(&mut header).map_err(|_| "更新包不完整")?;
    if &header[..2] != b"MZ" {
        return Err("更新包不是 Windows 程序".into());
    }
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap());
    if !(64..=1024 * 1024).contains(&offset) {
        return Err("更新包文件头无效".into());
    }
    file.seek(SeekFrom::Start(offset.into()))
        .map_err(|_| "更新包文件头无效")?;
    let mut signature = [0; 6];
    file.read_exact(&mut signature)
        .map_err(|_| "更新包文件头无效")?;
    if &signature[..4] != b"PE\0\0"
        || !matches!(
            u16::from_le_bytes([signature[4], signature[5]]),
            0x8664 | 0x14c
        )
    {
        return Err("更新包不是受支持的 Windows 程序".into());
    }
    Ok(())
}

pub(super) fn download(
    release: &Release,
    target: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(u64),
) -> Result<Prepared, String> {
    let parent = target.parent().ok_or("无法确定程序目录")?;
    let directory = tempfile::Builder::new()
        .prefix(".zhku-update-")
        .tempdir_in(parent)
        .map_err(|_| "程序目录不可写，请打开发布页面手动更新")?;
    let mut response = client()?
        .get(&release.url)
        .send()
        .map_err(|_| "下载失败，请检查网络或代理")?;
    if !response.status().is_success() {
        return Err("下载更新包失败，请稍后重试".into());
    }
    if let Some(size) = response.content_length()
        && size != release.size
    {
        return Err("更新包大小与发布信息不符".into());
    }
    let payload = directory.path().join("payload.exe");
    let mut file = File::create(&payload).map_err(|_| "无法创建更新包文件")?;
    let mut bytes = 0u64;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("下载已取消".into());
        }
        let n = response.read(&mut buffer).map_err(|_| "下载中断，请重试")?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > release.size {
            return Err("更新包超过预期大小".into());
        }
        file.write_all(&buffer[..n]).map_err(|_| "无法保存更新包")?;
        hash.update(&buffer[..n]);
        progress(bytes);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("下载已取消".into());
    }
    if bytes != release.size || format!("{:x}", hash.finalize()) != release.sha256 {
        return Err("更新包校验失败，已停止更新".into());
    }
    file.sync_all().map_err(|_| "无法保存更新包")?;
    drop(file);
    verify_pe(&payload)?;
    Ok(Prepared {
        directory,
        target: target.into(),
        hash: release.sha256.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashing_and_executable_validation_reject_html_and_truncated_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("payload.exe");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(
            hash_file(&file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(verify_pe(&file).is_err());
        std::fs::write(&file, b"<html>not an executable</html>").unwrap();
        assert!(verify_pe(&file).is_err());
    }
}
