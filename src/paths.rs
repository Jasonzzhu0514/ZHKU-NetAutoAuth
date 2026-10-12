use std::path::PathBuf;

/// Shared application directory; credential and desktop settings stay separate.
pub(crate) fn app_data() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(|value| {
            PathBuf::from(value)
                .join("ZHKU-NetAutoAuth")
                .join("rust-gui")
        })
        .ok_or_else(|| "无法确定 Windows 本地应用数据目录".into())
}
