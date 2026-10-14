use super::model::{Target, TestResult};
use super::{connection, route};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn run(target: &Target) -> TestResult {
    let (latency_ms, failure) = match measure(&target.url) {
        Ok(latency) => (Some(latency), None),
        Err(error) => (None, Some(error)),
    };
    TestResult {
        tested_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        latency_ms,
        failure,
    }
}

fn measure(raw_url: &str) -> Result<u64, String> {
    let url = reqwest::Url::parse(raw_url).map_err(|_| "网址格式错误")?;
    let route = route::for_url(&url)?;
    let config = connection::tls_config();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "无法初始化连接测试")?;
    let result = runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(8),
            connection::measure(&url, route.as_ref(), config),
        )
        .await
    });
    // A cancelled OS DNS lookup must not hold the caller past the timeout.
    runtime.shutdown_background();
    result.map_err(|_| "连接超时".to_owned())?
}
