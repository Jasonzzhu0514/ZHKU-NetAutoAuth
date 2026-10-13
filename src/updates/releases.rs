use reqwest::{Url, blocking::Client};
use semver::Version;
use serde_json::Value;
use std::{io::Read, time::Duration};

pub(super) const REPOSITORY: &str = "Jasonzzhu0514/ZHKU-NetAutoAuth";
pub(super) const RELEASES_URL: &str = "https://github.com/Jasonzzhu0514/ZHKU-NetAutoAuth/releases";
pub(super) const MAX_PACKAGE: u64 = 100 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum InstallMode {
    Installed,
    #[default]
    Portable,
}

#[derive(Clone)]
pub(super) struct Release {
    pub version: Version,
    pub notes: String,
    pub page: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

pub(super) enum Check {
    Current(Version),
    Update(Release),
}

pub(super) fn client() -> Result<Client, String> {
    Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("ZHKU-NetAutoAuth/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("redirect limit")
            } else if trusted_host(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("untrusted update redirect")
            }
        }))
        .build()
        .map_err(|_| "无法初始化更新连接".into())
}

fn trusted_host(url: &Url) -> bool {
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
                    | "github-releases.githubusercontent.com"
            )
        )
}

pub(super) fn valid_asset_url(value: &str, tag: &str, name: &str) -> bool {
    let expected = format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}");
    value == expected
}

fn version(value: &Value) -> Result<(Version, &str), String> {
    if value["draft"].as_bool() != Some(false) || value["prerelease"].as_bool() != Some(false) {
        return Err("仅支持正式发布版本".into());
    }
    let tag = value["tag_name"].as_str().ok_or("发布版本无效")?;
    let version =
        Version::parse(tag.strip_prefix('v').ok_or("发布版本无效")?).map_err(|_| "发布版本无效")?;
    if !version.pre.is_empty() || !version.build.is_empty() || version.major < 2 {
        return Err("仅支持 Rust 桌面正式版本".into());
    }
    Ok((version, tag))
}

pub(super) fn parse(value: &Value, mode: InstallMode) -> Result<Release, String> {
    let (version, tag) = version(value)?;
    let name = match mode {
        InstallMode::Installed => "ZHKU-NetAutoAuth-Setup.exe",
        InstallMode::Portable => "ZHKU-NetAutoAuth.exe",
    };
    let assets = value["assets"].as_array().ok_or("未找到更新附件")?;
    let mut matching = assets
        .iter()
        .filter(|asset| asset["name"].as_str() == Some(name));
    let asset = matching
        .next()
        .ok_or("该版本没有对应的更新包，请打开发布页面手动下载")?;
    if matching.next().is_some() {
        return Err("更新附件重复".into());
    }
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("更新地址无效")?;
    if !valid_asset_url(url, tag, name) {
        return Err("更新地址不属于官方发布源".into());
    }
    let size = asset["size"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= MAX_PACKAGE)
        .ok_or("更新包大小无效")?;
    let hash = asset["digest"]
        .as_str()
        .and_then(|v| v.strip_prefix("sha256:"))
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("发布附件缺少 SHA-256 校验信息，请手动下载")?;
    let page = format!("{RELEASES_URL}/tag/{tag}");
    Ok(Release {
        version,
        notes: value["body"].as_str().unwrap_or("暂无更新说明").into(),
        page,
        url: url.into(),
        size,
        sha256: hash.to_ascii_lowercase(),
    })
}

fn check(value: &Value, mode: InstallMode, current: &Version) -> Result<Check, String> {
    let (latest, _) = version(value)?;
    if latest <= *current {
        return Ok(Check::Current(latest));
    }
    parse(value, mode).map(Check::Update)
}

pub(super) fn latest(mode: InstallMode) -> Result<Check, String> {
    let mut response = client()?
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(15))
        .send()
        .map_err(|_| "无法连接 GitHub，请检查网络或代理后重试")?;
    if response.status().as_u16() == 403 || response.status().as_u16() == 429 {
        return Err("GitHub 请求受限，请稍后再试".into());
    }
    if !response.status().is_success() {
        return Err("无法获取发布信息，请稍后再试".into());
    }
    let mut bytes = Vec::new();
    (&mut response)
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取发布信息")?;
    if bytes.len() > 1024 * 1024 {
        return Err("发布信息过大".into());
    }
    check(
        &serde_json::from_slice(&bytes).map_err(|_| "发布信息格式无效")?,
        mode,
        &Version::parse(env!("CARGO_PKG_VERSION")).expect("package version"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release() -> Value {
        serde_json::json!({"tag_name":"v2.10.0","draft":false,"prerelease":false,"body":"notes", "assets":[{
            "name":"ZHKU-NetAutoAuth.exe","size":1000,"digest":format!("sha256:{}", "a".repeat(64)),
            "browser_download_url":"https://github.com/Jasonzzhu0514/ZHKU-NetAutoAuth/releases/download/v2.10.0/ZHKU-NetAutoAuth.exe"
        }]})
    }
    #[test]
    fn versions_are_semantic_and_only_official_verified_assets_are_accepted() {
        let r = parse(&release(), InstallMode::Portable).unwrap();
        assert!(r.version > Version::parse("2.9.0").unwrap());
        for (field, value) in [
            (
                "browser_download_url",
                Value::String("https://example.com/client.exe".into()),
            ),
            ("digest", Value::Null),
            ("size", Value::from(MAX_PACKAGE + 1)),
        ] {
            let mut v = release();
            v["assets"][0][field] = value;
            assert!(parse(&v, InstallMode::Portable).is_err());
        }
        assert!(parse(&release(), InstallMode::Installed).is_err());
    }
    #[test]
    fn older_release_does_not_require_installer_asset() {
        let mut old = release();
        old["assets"] = serde_json::json!([]);
        assert!(matches!(
            check(
                &old,
                InstallMode::Installed,
                &Version::parse("2.11.0").unwrap()
            )
            .unwrap(),
            Check::Current(_)
        ));
        assert!(
            check(
                &old,
                InstallMode::Installed,
                &Version::parse("2.9.0").unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn draft_prerelease_legacy_and_duplicate_assets_are_rejected() {
        for field in ["draft", "prerelease"] {
            let mut v = release();
            v[field] = Value::Bool(true);
            assert!(parse(&v, InstallMode::Portable).is_err());
        }
        for tag in ["v1.0.1", "v2.2.0-beta.1", "invalid"] {
            let mut v = release();
            v["tag_name"] = Value::String(tag.into());
            assert!(parse(&v, InstallMode::Portable).is_err());
        }
        let mut v = release();
        let asset = v["assets"][0].clone();
        v["assets"].as_array_mut().unwrap().push(asset);
        assert!(parse(&v, InstallMode::Portable).is_err());
    }
}
