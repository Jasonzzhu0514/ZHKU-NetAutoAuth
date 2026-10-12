use super::*;

#[test]
fn status_exposes_server_mac_and_vlan_without_substituting_ss4() {
    let status=Portal::decode_status(&serde_json::json!({"result":1,"olmac":"020000000001","ss4":"changing-marker","cvid":1300,"pvid":0})).unwrap();
    assert_eq!(status.online_mac, "020000000001");
    assert_eq!(status.cvid, "1300");
    assert_eq!(status.pvid, "0");
    let absent =
        Portal::decode_status(&serde_json::json!({"result":1,"ss4":"changing-marker"})).unwrap();
    assert!(absent.online_mac.is_empty());
}

#[test]
fn raw_diagnostic_messages_are_preserved_without_display_truncation() {
    let (tx, rx) = std::sync::mpsc::channel();
    let portal = Portal::new().unwrap().with_trace(tx);
    let message = "diagnostic-content ".repeat(1000);
    portal.trace(&message, false);
    assert_eq!(rx.recv().unwrap().message, message);
}
#[test]
fn raw_log_redacts_password_fields_and_echoed_secrets() {
    let secrets = vec![
        Zeroizing::new("s3cr&et".into()),
        Zeroizing::new("s3cr%26et".into()),
    ];
    let raw = r#"campusAuth({"result":0,"password":"other-secret","token":"private-token","message":"s3cr&et s3cr%26et"});"#;
    let safe = redact(raw, &secrets);
    for secret in ["other-secret", "private-token", "s3cr&et", "s3cr%26et"] {
        assert!(!safe.contains(secret));
    }
    assert!(safe.contains("\"result\":0"));
    assert!(safe.contains("[REDACTED]"));
}

#[test]
fn network_trace_contains_http_status_and_raw_response_without_password() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", server.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut connection, _) = server.accept().unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = [0; 4096];
        let count = connection.read(&mut request).unwrap();
        assert!(count > 0);
        let body = r#"campusAuth({"result":1,"password":"test-secret"});"#;
        write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let (tx, rx) = std::sync::mpsc::channel();
    let portal = Portal::new().unwrap().with_trace(tx);
    portal
        .request(
            &origin,
            "/example",
            &[("upass".into(), "test-secret".into())],
        )
        .unwrap();
    worker.join().unwrap();
    let log = rx
        .try_iter()
        .map(|t| t.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(log.contains("REQUEST GET"));
    assert!(log.contains("HTTP=200"));
    assert!(log.contains("campusAuth("));
    assert!(!log.contains("test-secret"));
}
#[test]
fn jsonp_is_data() {
    assert_eq!(
        parse_jsonp("campusAuth({\"result\":1});").unwrap()["result"],
        1
    );
    assert!(parse_jsonp("campusAuth({});alert(1)").is_err());
    assert!(parse_jsonp("campusAuth([])").is_err());
    assert!(parse_jsonp("<html>Error</html>").is_err());
}
#[test]
fn login_query_keeps_verified_order() {
    let q = Portal::query(vec![("DDDDD", "test"), ("upass", "test")]);
    assert_eq!(
        q.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
        vec!["callback", "DDDDD", "upass", "jsVersion", "v", "lang"]
    );
}

#[test]
fn unknown_status_is_never_treated_as_offline() {
    for data in [
        serde_json::json!({}),
        serde_json::json!({"result": 2}),
        serde_json::json!({"result": false}),
    ] {
        assert!(Portal::decode_status(&data).is_err());
    }
    assert_eq!(
        Portal::decode_status(&serde_json::json!({"result": "0"}))
            .unwrap()
            .state,
        State::Offline
    );
}

#[test]
fn online_status_accepts_numeric_result_and_ip_fallback() {
    let data = serde_json::json!({"result": 1, "ss5": "192.0.2.1", "uid": "example"});
    let status = Portal::decode_status(&data).unwrap();
    assert_eq!(status.state, State::Online);
    assert_eq!(status.ip, "192.0.2.1");
    assert_eq!(status.account, "example");
}

fn test_status(result: u8) -> Status {
    Portal::decode_status(&serde_json::json!({"result": result})).unwrap()
}

#[test]
fn valid_status_does_not_depend_on_landing_page_probe() {
    let status = Portal::detect_status(
        Ok(test_status(1)),
        || panic!("no page probe needed"),
        || panic!("no dormitory probe needed"),
    );
    assert_eq!(status.state, State::Online);
}

#[test]
fn network_failure_preserves_reason_instead_of_claiming_disconnected() {
    let status = Portal::detect_status(Err("timeout".into()), || false, || false);
    assert_eq!(status.state, State::Unknown);
    assert!(status.detail.contains("timeout"));
    assert!(!status.detail.contains("请先连接"));
}

#[test]
fn reachable_office_with_invalid_api_stays_unknown() {
    let status = Portal::detect_status(
        Err("invalid JSON".into()),
        || true,
        || panic!("must not switch region"),
    );
    assert_eq!(status.region, "办公区");
    assert_eq!(status.state, State::Unknown);
}

#[test]
fn logout_unknown_or_offline_does_not_send_request() {
    let unknown = Status::unknown("宿舍区", "待适配");
    assert!(
        Portal::logout_checked(
            unknown,
            || panic!("must not request"),
            || panic!("must not check"),
            || {}
        )
        .is_err()
    );
    let offline = Portal::logout_checked(
        test_status(0),
        || panic!("must not request"),
        || panic!("must not check"),
        || {},
    )
    .unwrap();
    assert_eq!(offline.state, State::Offline);
}

#[test]
fn logout_acknowledgement_alone_is_not_success() {
    let result = Portal::logout_checked(test_status(1), || Ok(()), || Ok(test_status(1)), || {});
    assert!(result.is_err());
}

#[test]
fn logout_verifies_offline_even_if_reply_is_unrecognizable() {
    let mut checks = 0;
    let result = Portal::logout_checked(
        test_status(1),
        || Err("invalid reply".into()),
        || {
            checks += 1;
            Ok(test_status(if checks < 2 { 1 } else { 0 }))
        },
        || {},
    )
    .unwrap();
    assert_eq!(checks, 2);
    assert_eq!(result.state, State::Offline);
}
