use super::model::{MAX_TARGETS, Target, defaults, normalize};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::Path,
};

pub fn load(path: &Path) -> Result<Vec<Target>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(defaults()),
        Err(_) => return Err("无法读取网站列表".into()),
    };
    let mut data = Vec::new();
    file.take(65537)
        .read_to_end(&mut data)
        .map_err(|_| "无法读取网站列表")?;
    if data.len() > 65536 {
        return Err("网站列表文件过大".into());
    }
    let targets: Vec<Target> = serde_json::from_slice(&data).map_err(|_| "网站列表格式异常")?;
    validate(&targets)?;
    Ok(targets)
}

fn validate(targets: &[Target]) -> Result<(), String> {
    if targets.len() > MAX_TARGETS {
        return Err("最多保存 32 个网站".into());
    }
    let mut ids = HashSet::new();
    let mut urls = HashSet::new();
    for target in targets {
        let (_, url) = normalize(&target.name, &target.url)?;
        if target.id == 0 || !ids.insert(target.id) || !urls.insert(url) {
            return Err("网站列表包含重复网址或无效标识".into());
        }
    }
    Ok(())
}

pub fn save(path: &Path, targets: &[Target]) -> Result<(), String> {
    validate(targets)?;
    let parent = path.parent().ok_or("网站列表目录无效")?;
    std::fs::create_dir_all(parent).map_err(|_| "无法创建网站列表目录")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法创建网站列表文件")?;
    let data = serde_json::to_vec_pretty(targets).map_err(|_| "无法保存网站列表")?;
    file.write_all(&data)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "无法写入网站列表")?;
    file.persist(path)
        .map_err(|_| "无法替换网站列表文件，请检查权限")?;
    Ok(())
}
