use super::*;

#[test]
fn url_validation_and_roundtrip_preserve_custom_and_empty_lists() {
    for invalid in [
        "file:///tmp/file",
        "https://user:secret@example.test",
        "https://example.test/#token",
        "invalid",
    ] {
        assert!(normalize("test", invalid).is_err());
    }
    assert!(normalize("", "https://example.test/").is_err());
    assert_eq!(
        normalize(" test ", "https://example.test").unwrap(),
        ("test".into(), "https://example.test/".into())
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("network-tests.json");
    assert_eq!(config::load(&path).unwrap().len(), 4);
    let mut service = Service {
        path: Some(path.clone()),
        ..Service::default()
    };
    service.add("test", "https://example.test").unwrap();
    assert_eq!(config::load(&path).unwrap().len(), 5);
    assert!(service.add("duplicate", "https://example.test/").is_err());
    let ids: Vec<_> = service
        .entries
        .iter()
        .map(|entry| entry.target.id)
        .collect();
    for id in ids {
        service.remove(id).unwrap();
    }
    assert!(config::load(&path).unwrap().is_empty());
}

#[test]
fn failed_save_keeps_original_targets_and_rejects_corrupt_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("network-tests.json");
    std::fs::create_dir(&path).unwrap();
    let mut service = Service {
        path: Some(path),
        ..Service::default()
    };
    assert!(service.add("test", "https://example.test/").is_err());
    assert!(service.remove(1).is_err());
    assert_eq!(service.entries.len(), 4);
    let corrupt = dir.path().join("corrupt.json");
    std::fs::write(&corrupt, "[{}").unwrap();
    assert!(config::load(&corrupt).is_err());
    std::fs::write(
        &corrupt,
        serde_json::to_vec(&vec![Target {
            id: 1,
            name: "test".into(),
            url: "file:///tmp/test".into(),
        }])
        .unwrap(),
    )
    .unwrap();
    assert!(config::load(&corrupt).is_err());
}

#[test]
fn independent_results_survive_other_tests_and_busy_blocks_list_edits() {
    let mut service = Service::default();
    let (sender, receiver) = mpsc::channel();
    service.receiver = Some(receiver);
    service.pending = vec![1, 2];
    assert!(service.start(Some(1)).is_err());
    assert!(service.add("test", "https://example.test/").is_err());
    assert!(service.remove(1).is_err());
    sender
        .send((
            1,
            TestResult {
                tested_at_ms: 1700000000000,
                latency_ms: Some(20),
                failure: None,
            },
        ))
        .unwrap();
    assert!(service.poll());
    assert_eq!(service.pending, vec![2]);
    assert!(service.snapshot().busy);
    sender
        .send((
            2,
            TestResult {
                tested_at_ms: 1700000001000,
                latency_ms: None,
                failure: Some("timeout".into()),
            },
        ))
        .unwrap();
    drop(sender);
    assert!(service.poll());
    assert!(!service.snapshot().busy);
    assert!(service.pending.is_empty());
    assert_eq!(
        service.entries[0].result.as_ref().unwrap().latency_ms,
        Some(20)
    );
    assert!(
        service.entries[1]
            .result
            .as_ref()
            .unwrap()
            .failure
            .is_some()
    );
    assert!(service.entries[2].result.is_none());
}

#[test]
fn starting_tests_clears_only_the_selected_previous_results() {
    let mut service = Service::default();
    for entry in &mut service.entries {
        // Closed loopback port: no public websites or user data are used by this test.
        entry.target.url = "http://127.0.0.1:1/".into();
        entry.result = Some(TestResult {
            tested_at_ms: 1700000000000,
            latency_ms: Some(20),
            failure: None,
        });
    }
    service.start(Some(2)).unwrap();
    assert!(service.entries[1].result.is_none());
    assert!(service.entries[0].result.is_some());
    assert!(service.entries[2].result.is_some());
    assert!(service.entries[3].result.is_some());
    assert_eq!(service.pending, vec![2]);
    // Without polling, completed worker messages do not change the service's state.
    service.receiver = None;
    service.start(None).unwrap();
    assert!(service.entries.iter().all(|entry| entry.result.is_none()));
}
