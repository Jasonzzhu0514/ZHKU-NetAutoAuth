use super::*;
use zeroize::Zeroizing;

fn configured() -> Client {
    Client {
        settings: storage::Settings {
            username: "test-account".into(),
            password: Zeroizing::new("test-only-secret".into()),
            remember: false,
            auto_login: false,
        },
        ..Client::default()
    }
}

#[test]
fn state_events_never_include_passwords_or_credential_fields() {
    let client = configured();
    let event = serde_json::to_value(client.snapshot()).unwrap();
    assert!(event.get("password").is_none());
    assert!(!event.to_string().contains("test-only-secret"));
    assert_eq!(event["hasPassword"], true);
}

#[test]
fn guard_requires_credentials_and_does_not_enable_password_storage() {
    let mut empty = Client::default();
    assert!(empty.set_guard(true).is_err());
    assert!(!empty.guard);
    let mut client = configured();
    client.set_guard(true).unwrap();
    assert!(client.guard);
    assert!(!client.settings.remember);
    assert!(!client.settings.auto_login);
    assert!(!client.resume_guard_after_update());
    client.set_guard(false).unwrap();
    assert!(!client.guard);
}

#[test]
fn guard_can_toggle_during_a_request_without_cancelling_it() {
    let mut client = configured();
    let (_sender, receiver) = mpsc::channel();
    client.receiver = Some(receiver);
    client.failures = 4;
    client.set_guard(true).unwrap();
    assert!(client.guard);
    assert!(client.guard_pending);
    assert_eq!(client.failures, 0);
    assert!(client.receiver.is_some());
    client.set_guard(false).unwrap();
    assert!(!client.guard);
    assert!(!client.guard_pending);
    assert!(client.receiver.is_some());
    client.tick();
    assert!(!client.guard);
    assert!(client.receiver.is_some());
}

#[test]
fn snapshots_advance_revision_on_busy_guard_and_log_changes() {
    let mut client = configured();
    let initial = client.snapshot().revision;
    client.set_guard(true).unwrap();
    let guarding = client.snapshot().revision;
    assert!(guarding > initial);
    client.set_guard(false).unwrap();
    client.clear_logs();
    assert!(client.snapshot().revision > guarding);
    let snapshot = client.snapshot().revision;
    client.take_changed();
    assert_eq!(client.snapshot().revision, snapshot);
}

#[test]
fn interval_change_persists_and_reschedules_without_interrupting_request() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("desktop.json");
    let mut client = configured();
    let (sender, receiver) = mpsc::channel();
    client.receiver = Some(receiver);
    client.guard = true;
    client.guard_pending = true;
    let mut input = client.snapshot().preferences;
    input.guard_interval_seconds = 90;
    client.set_preferences_at(input, &path).unwrap();
    assert_eq!(
        preferences::Preferences::load(&path)
            .unwrap()
            .guard_interval_seconds,
        90
    );
    assert!(client.receiver.is_some());
    assert!(!client.guard_pending);
    let delay = client
        .next_check
        .saturating_duration_since(Instant::now())
        .as_secs();
    assert!((89..=90).contains(&delay));
    sender
        .send(NetworkEvent {
            status: portal::Status {
                state: portal::State::Online,
                ..portal::Status::unknown("测试区域", "")
            },
            message: "已通过认证".into(),
            failed: false,
        })
        .unwrap();
    client.tick();
    assert!(client.receiver.is_none());
    assert!(client.guard);
    let delay = client
        .next_check
        .saturating_duration_since(Instant::now())
        .as_secs();
    assert!((89..=90).contains(&delay));
}

#[test]
fn locked_accounts_cannot_delete_or_replace_saved_credentials() {
    let mut client = configured();
    client.guard = true;
    assert!(client.forget_account().is_err());
    assert!(
        client
            .set_account(AccountInput {
                username: "replacement".into(),
                password: "replacement".into(),
                remember_password: true,
                auto_login: true,
            })
            .is_err()
    );
    assert_eq!(client.settings.username, "test-account");
    assert_eq!(client.settings.password.as_str(), "test-only-secret");
}

#[test]
fn paged_logs_preserve_raw_messages_and_clear_invalidates_old_history() {
    let mut client = Client::default();
    for index in 0..600 {
        client.log(format!("RESPONSE {index}\n原始正文"), index % 2 == 0);
    }
    let first = client.logs_after(0);
    assert_eq!(first.len(), 256);
    assert!(first[0].message.ends_with("\n原始正文"));
    let second = client.logs_after(first.last().unwrap().id);
    assert_eq!(second.first().unwrap().id, first.last().unwrap().id + 1);
    client.clear_logs();
    assert!(client.logs_after(0).is_empty());
    assert_eq!(client.snapshot().log_generation, 1);
    client.log("new entry", false);
    assert!(client.logs[0].id > second.last().unwrap().id);
}

#[test]
fn worker_completion_preserves_traces_and_backoff_without_network_requests() {
    let mut client = Client::default();
    for (failed, seconds) in [
        (false, 30),
        (true, 60),
        (true, 120),
        (true, 240),
        (true, 300),
        (true, 300),
    ] {
        let (sender, receiver) = mpsc::channel();
        let (trace_sender, traces) = mpsc::channel();
        client.receiver = Some(receiver);
        client.trace_receiver = Some(traces);
        trace_sender
            .send(portal::Trace {
                at: Instant::now(),
                message: "raw trace".into(),
                failed,
            })
            .unwrap();
        sender
            .send(NetworkEvent {
                status: if failed {
                    portal::Status::unknown("未检测到", "timeout")
                } else {
                    portal::Status {
                        state: portal::State::Online,
                        ..portal::Status::unknown("测试区域", "")
                    }
                },
                message: if failed { "timeout" } else { "已通过认证" }.into(),
                failed,
            })
            .unwrap();
        client.tick();
        let delay = client
            .next_check
            .saturating_duration_since(Instant::now())
            .as_secs();
        assert!((seconds - 1..=seconds).contains(&delay));
        assert!(client.receiver.is_none());
        assert_eq!(client.logs[client.logs.len() - 2].message, "raw trace");
    }
}
