use crate::portal_format::{CALLBACK, field, parse_jsonp, redact};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    cell::RefCell,
    sync::mpsc::Sender,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

// Campus server defaults; no personal device or account data.
const OFFICE: &str = "http://10.255.255.254";
const DORMITORY: &str = "http://192.16.99.5";

pub struct Credentials {
    pub username: String,
    pub password: Zeroizing<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Online,
    Offline,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub state: State,
    pub region: &'static str,
    pub ip: String,
    pub account: String,
    pub online_mac: String,
    pub cvid: String,
    pub pvid: String,
    pub detail: String,
}

impl Status {
    pub fn unknown(region: &'static str, detail: impl Into<String>) -> Self {
        Self {
            state: State::Unknown,
            region,
            ip: String::new(),
            account: String::new(),
            online_mac: String::new(),
            cvid: String::new(),
            pvid: String::new(),
            detail: detail.into(),
        }
    }
}

pub struct Portal {
    client: Client,
    trace_sink: Option<Sender<Trace>>,
    secrets: RefCell<Vec<Zeroizing<String>>>,
}

pub struct Trace {
    pub at: Instant,
    pub message: String,
    pub failed: bool,
}

impl Portal {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent(concat!("ZHKU-NetAutoAuth/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "无法初始化网络客户端")?;
        Ok(Self {
            client,
            trace_sink: None,
            secrets: RefCell::new(Vec::new()),
        })
    }

    pub fn with_trace(mut self, sink: Sender<Trace>) -> Self {
        self.trace_sink = Some(sink);
        self
    }

    fn trace(&self, message: impl Into<String>, failed: bool) {
        if let Some(sink) = &self.trace_sink {
            let message = redact(&message.into(), &self.secrets.borrow());
            let _ = sink.send(Trace {
                at: Instant::now(),
                message,
                failed,
            });
        }
    }

    fn reachable(&self, origin: &str) -> bool {
        let started = Instant::now();
        self.trace(format!("PROBE GET {origin} timeout=2s"), false);
        let result = self
            .client
            .get(origin)
            .timeout(Duration::from_secs(2))
            .send();
        match result {
            Ok(response) => {
                let reachable =
                    response.status().is_success() || response.status().is_redirection();
                self.trace(
                    format!(
                        "PROBE {origin} HTTP={} elapsed={}ms",
                        response.status().as_u16(),
                        started.elapsed().as_millis()
                    ),
                    !reachable,
                );
                reachable
            }
            Err(error) => {
                self.trace(
                    format!(
                        "PROBE {origin} error={} elapsed={}ms",
                        error.without_url(),
                        started.elapsed().as_millis()
                    ),
                    true,
                );
                false
            }
        }
    }

    fn request(
        &self,
        origin: &str,
        path: &str,
        query: &[(String, String)],
    ) -> Result<String, String> {
        let started = Instant::now();
        let safe_query: Vec<_> = query
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str(),
                    if matches!(k.as_str(), "upass" | "DDDDD") {
                        "[REDACTED]"
                    } else {
                        v.as_str()
                    },
                )
            })
            .collect();
        self.trace(
            format!("REQUEST GET {origin}{path} params={safe_query:?}"),
            false,
        );
        // Never expose reqwest errors: their URL may contain the password.
        let response = self
            .client
            .get(format!("{origin}{path}"))
            .header("Referer", format!("{OFFICE}/a79.htm"))
            .header("Cache-Control", "no-cache")
            .query(query)
            .send()
            .map_err(|error| {
                let timed_out = error.is_timeout();
                let connect = error.is_connect();
                let error = error.without_url();
                let mut detail = error.to_string();
                let mut source = std::error::Error::source(&error);
                for _ in 0..4 {
                    if let Some(cause) = source { detail.push_str(&format!("; caused by: {cause}")); source = cause.source(); } else { break; }
                }
                self.trace(format!("ERROR {origin}{path} elapsed={}ms timeout={timed_out} connect={connect} {detail}", started.elapsed().as_millis()), true);
                if error.is_timeout() {
                    "认证服务器响应超时，请稍后刷新"
                } else if error.is_connect() {
                    "无法连接认证服务器，请检查当前网络或 VPN 路由"
                } else {
                    "认证服务器请求失败，请稍后刷新"
                }
            })?;
        let status = response.status();
        // Bound the response size, including decompressed responses.
        let mut response = response;
        let mut bytes = Vec::new();
        use std::io::Read;
        (&mut response)
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                self.trace(format!("READ ERROR {origin}{path}: {error}"), true);
                "无法读取服务器响应"
            })?;
        if bytes.len() > 1024 * 1024 {
            return Err("服务器响应过大".into());
        }
        let body = String::from_utf8_lossy(&bytes).into_owned();
        self.trace(
            format!(
                "RESPONSE {origin}{path} HTTP={} elapsed={}ms bytes={}\n{body}",
                status.as_u16(),
                started.elapsed().as_millis(),
                bytes.len()
            ),
            !status.is_success(),
        );
        if !status.is_success() {
            return Err(format!("认证服务器返回 HTTP {}", status.as_u16()));
        }
        Ok(body)
    }

    fn query(params: Vec<(&str, &str)>) -> Vec<(String, String)> {
        // Keep the order required by the verified office login parser.
        let mut query = vec![("callback".into(), CALLBACK.into())];
        query.extend(params.into_iter().map(|(k, v)| (k.into(), v.into())));
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            % 10000
            + 500;
        query.extend([
            ("jsVersion".into(), "4.X".into()),
            ("v".into(), nonce.to_string()),
            ("lang".into(), "zh".into()),
        ]);
        query
    }

    fn api(&self, origin: &str, path: &str, params: Vec<(&str, &str)>) -> Result<Value, String> {
        let result = parse_jsonp(&self.request(origin, path, &Self::query(params))?);
        if let Err(error) = &result {
            self.trace(format!("PARSE ERROR {path}: {error}"), true);
        }
        result
    }

    fn office_status(&self) -> Result<Status, String> {
        let data = self.api(OFFICE, "/drcom/chkstatus", vec![])?;
        Self::decode_status(&data)
    }

    fn decode_status(data: &Value) -> Result<Status, String> {
        let state = match field(data, "result").as_str() {
            "1" => State::Online,
            "0" => State::Offline,
            _ => return Err("在线接口未返回有效状态，本轮不提交认证".into()),
        };
        let ip = ["v4ip", "v46ip", "ss5"]
            .into_iter()
            .map(|k| field(data, k))
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        Ok(Status {
            state,
            region: "办公区",
            ip,
            account: field(data, "uid"),
            online_mac: field(data, "olmac"),
            cvid: field(data, "cvid"),
            pvid: field(data, "pvid"),
            detail: String::new(),
        })
    }

    pub fn status(&self) -> Status {
        Self::detect_status(
            self.office_status(),
            || self.reachable(OFFICE),
            || self.reachable(DORMITORY),
        )
    }

    fn detect_status(
        office: Result<Status, String>,
        office_reachable: impl FnOnce() -> bool,
        dormitory_reachable: impl FnOnce() -> bool,
    ) -> Status {
        // The authentication endpoint itself proves office reachability.
        // A slow or unavailable landing page must not hide a valid online state.
        match office {
            Ok(status) => status,
            Err(detail) if office_reachable() => Status::unknown("办公区", detail),
            Err(_) if dormitory_reachable() => {
                Status::unknown("宿舍区", "已识别宿舍区，认证功能尚待适配")
            }
            Err(detail) => Status::unknown(
                "暂无法确认",
                format!("{detail}；区域探测也未成功，请刷新重试"),
            ),
        }
    }

    pub fn login(&self, credentials: &Credentials) -> Result<Status, String> {
        // Include raw, URL-encoded and JSON-escaped representations to catch echoed credentials.
        let mut secrets = self.secrets.borrow_mut();
        for secret in [credentials.password.as_str(), credentials.username.as_str()] {
            if secret.is_empty() {
                continue;
            }
            secrets.push(Zeroizing::new(secret.to_owned()));
            let mut url = reqwest::Url::parse("http://localhost/").expect("static URL");
            url.query_pairs_mut().append_pair("v", secret);
            secrets.push(Zeroizing::new(
                url.query()
                    .unwrap_or_default()
                    .trim_start_matches("v=")
                    .to_owned(),
            ));
            let escaped = serde_json::to_string(secret).unwrap_or_default();
            secrets.push(Zeroizing::new(escaped.trim_matches('"').to_owned()));
        }
        drop(secrets);
        let current = self.status();
        match current.state {
            State::Online => return Ok(current),
            State::Unknown => return Err(current.detail),
            State::Offline => {}
        }
        let page = self.request(OFFICE, "/a79.htm", &[])?;
        let pattern =
            regex::Regex::new(r#"vlanid\s*=\s*['"]([^'"]*)"#).map_err(|_| "无法解析页面配置")?;
        let vlan = pattern
            .captures(&page)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("");
        let config = self.api(
            &format!("{OFFICE}:801"),
            "/eportal/portal/page/loadConfig",
            vec![
                ("program_index", ""),
                ("wlan_vlan_id", vlan),
                ("gw_id", "000000000000"),
            ],
        )?;
        let data = &config["data"];
        if !data.is_object() || field(data, "login_method") != "0" {
            return Err("服务器认证方式已变化，停止提交认证".into());
        }
        if !matches!(field(data, "en_md5").as_str(), "0" | "") {
            return Err("服务器启用了 MD5 登录，需要适配".into());
        }
        let reply = self.api(
            OFFICE,
            "/drcom/login",
            vec![
                ("DDDDD", &credentials.username),
                ("upass", &credentials.password),
                ("0MKKey", "123456"),
                ("R1", "0"),
                ("R2", ""),
                ("R3", "0"),
                ("R6", "0"),
                ("para", "00"),
                ("v6ip", ""),
                ("terminal_type", "1"),
            ],
        );
        // Always verify the online state instead of trusting the login reply.
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(1));
            }
            if let Ok(status) = self.office_status()
                && status.state == State::Online
            {
                return Ok(status);
            }
        }
        reply?;
        Err("认证后尚未确认在线，请检查账号密码或浏览器认证页面".into())
    }

    pub fn logout(&self) -> Result<Status, String> {
        Self::logout_checked(
            self.status(),
            || self.api(OFFICE, "/drcom/logout", vec![]).map(|_| ()),
            || self.office_status(),
            || std::thread::sleep(Duration::from_secs(1)),
        )
    }

    fn logout_checked(
        current: Status,
        request: impl FnOnce() -> Result<(), String>,
        mut check: impl FnMut() -> Result<Status, String>,
        mut wait: impl FnMut(),
    ) -> Result<Status, String> {
        match current.state {
            State::Offline => return Ok(current),
            State::Unknown => return Err(current.detail),
            State::Online => {}
        }
        // The office page's default_logout sends an empty data object.
        // No credentials or device identifiers are submitted for logout.
        let reply = request();
        for attempt in 0..3 {
            if attempt > 0 {
                wait();
            }
            if let Ok(status) = check()
                && status.state == State::Offline
            {
                return Ok(status);
            }
        }
        reply?;
        Err("注销后尚未确认离线，自动守护保持停止，请刷新状态后重试".into())
    }
}

#[cfg(test)]
#[path = "tests/portal.rs"]
mod tests;
