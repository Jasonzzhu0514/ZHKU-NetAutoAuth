use hyper_util::client::proxy::matcher::Matcher;

pub(super) struct ProxyRoute {
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub authorization: Option<String>,
}

pub(super) fn for_url(url: &reqwest::Url) -> Result<Option<ProxyRoute>, String> {
    let uri = url.as_str().parse().map_err(|_| "网址格式错误")?;
    let Some(proxy) = Matcher::from_system().intercept(&uri) else {
        return Ok(None);
    };
    let secure = match proxy.uri().scheme_str() {
        Some("http") => false,
        Some("https") => true,
        _ => return Err("当前代理类型不支持连接测试，请使用 HTTP 代理或 TUN 路由".into()),
    };
    let host = proxy
        .uri()
        .host()
        .ok_or("代理地址缺少主机名")?
        .trim_matches(['[', ']'])
        .to_owned();
    let port = proxy
        .uri()
        .port_u16()
        .unwrap_or(if secure { 443 } else { 80 });
    let authorization = proxy
        .basic_auth()
        .map(|value| value.to_str().map(str::to_owned))
        .transpose()
        .map_err(|_| "代理认证配置无效")?;
    Ok(Some(ProxyRoute {
        host,
        port,
        secure,
        authorization,
    }))
}
