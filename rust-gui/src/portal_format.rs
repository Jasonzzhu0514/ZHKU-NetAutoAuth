use serde_json::Value;
use zeroize::Zeroizing;

pub(crate) const CALLBACK: &str = "campusAuth";

pub(crate) fn parse_jsonp(text: &str) -> Result<Value, String> {
    let text = text.trim();
    let payload = if text.starts_with('{') {
        text
    } else {
        text.strip_prefix(CALLBACK)
            .map(str::trim)
            .and_then(|s| s.strip_prefix('('))
            .and_then(|s| s.trim_end_matches(';').trim_end().strip_suffix(')'))
            .ok_or("认证接口返回了无法识别的内容")?
    };
    let value: Value = serde_json::from_str(payload).map_err(|_| "认证接口返回了无效 JSON")?;
    if !value.is_object() {
        return Err("认证接口返回格式异常".into());
    }
    Ok(value)
}

pub(crate) fn field(data: &Value, name: &str) -> String {
    match &data[name] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn redact(text: &str, secrets: &[Zeroizing<String>]) -> String {
    let fields = regex::Regex::new(
        r#"(?i)("(?:upass|password|passwd|token|authorization|cookie)"\s*:\s*)"(?:\\.|[^"\\])*""#,
    )
    .expect("valid redaction pattern");
    let mut value = fields
        .replace_all(text, |caps: &regex::Captures<'_>| {
            format!("{}\"[REDACTED]\"", &caps[1])
        })
        .into_owned();
    for secret in secrets {
        if !secret.is_empty() {
            value = value.replace(secret.as_str(), "[REDACTED]");
        }
    }
    value
}
