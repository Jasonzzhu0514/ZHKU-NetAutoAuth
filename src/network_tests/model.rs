use serde::{Deserialize, Serialize};

pub const MAX_TARGETS: usize = 32;

#[derive(Clone, Deserialize, Serialize)]
pub struct Target {
    pub id: u64,
    pub name: String,
    pub url: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub tested_at_ms: u64,
    pub latency_ms: Option<u64>,
    pub failure: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct Entry {
    pub target: Target,
    pub result: Option<TestResult>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub entries: Vec<Entry>,
    pub pending: Vec<u64>,
    pub busy: bool,
    pub warning: Option<String>,
}

pub fn normalize(name: &str, url: &str) -> Result<(String, String), String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err("网站名称须为 1～60 个字符".into());
    }
    let parsed =
        reqwest::Url::parse(url.trim()).map_err(|_| "请输入完整的 http:// 或 https:// 网址")?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("仅支持 HTTP 或 HTTPS 网址".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("网址中不能包含账号或密码".into());
    }
    if parsed.fragment().is_some() {
        return Err("请移除网址中的 # 片段".into());
    }
    if parsed.as_str().len() > 1024 {
        return Err("网址不能超过 1024 字节".into());
    }
    Ok((name.to_owned(), parsed.to_string()))
}

pub fn defaults() -> Vec<Target> {
    [
        ("百度", "https://www.baidu.com/"),
        ("必应", "https://www.bing.com/"),
        ("Google", "https://www.google.com/"),
        ("YouTube", "https://www.youtube.com/"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, url))| Target {
        id: index as u64 + 1,
        name: name.into(),
        url: url.into(),
    })
    .collect()
}
