use super::*;

#[test]
fn status_metadata_refresh_does_not_move_control_cards() {
    for size in [[920.0, 760.0], [800.0, 760.0], [540.0, 1100.0]] {
        let ctx = context();
        let mut app = App::empty();
        app.status.state = State::Online;
        app.status.region = "办公区";
        app.status.ip = "192.0.2.123".into();
        app.status.account = "test-account".into();
        app.status.online_mac = "001122334455".into();
        app.status.cvid = "1".into();
        app.status.pvid = "1".into();
        frame(&mut app, &ctx, size);
        let before = cards(&frame(&mut app, &ctx, size));
        assert!(
            before.iter().all(|r| r.right() <= size[0] - 27.0),
            "overflow: {before:?}"
        );
        app.status.cvid = "4094".into();
        app.status.pvid = "4094".into();
        for _ in 0..3 {
            let after = cards(&frame(&mut app, &ctx, size));
            assert_eq!(
                before[1], after[1],
                "status metadata moved controls at {size:?}"
            );
        }
    }
}

#[test]
fn busy_controls_keep_hover_style_and_ignore_clicks() {
    let ctx = context();
    let mut app = App::empty();
    app.username = "test-account".into();
    app.password = Zeroizing::new("test-password".into());
    app.status.state = State::Online;
    app.guard = true;
    let output = frame(&mut app, &ctx, [920.0, 760.0]);
    let control = cards(&output)[1];
    let buttons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r)
                if control.contains(r.rect.center()) && (r.rect.height() - 36.0).abs() < 1.0 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    for rect in buttons {
        app.receiver = None;
        let pointer = egui::Event::PointerMoved(rect.center());
        input_frame(&mut app, &ctx, [920.0, 760.0], vec![pointer.clone()]);
        let idle = input_frame(&mut app, &ctx, [920.0, 760.0], vec![pointer.clone()]);
        let (_tx, rx) = mpsc::channel();
        app.receiver = Some(rx);
        let querying = input_frame(&mut app, &ctx, [920.0, 760.0], vec![pointer]);
        let appearance = |out: &egui::FullOutput| {
            out.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.rect == rect => Some((r.fill, r.stroke)),
                _ => None,
            })
        };
        assert_eq!(appearance(&idle), appearance(&querying));
        for pressed in [true, false] {
            input_frame(
                &mut app,
                &ctx,
                [920.0, 760.0],
                vec![egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        assert!(app.guard, "busy logout must not stop guarding");
        assert!(app.logs.is_empty(), "busy clicks must not launch actions");
        assert!(app.receiver.is_some());
    }
}

#[test]
fn refreshing_preserves_guard_card_text_and_geometry() {
    let ctx = context();
    let mut app = App::empty();
    app.username = "test-account".into();
    app.password = Zeroizing::new("test-password".into());
    app.status.state = State::Online;
    app.guard = true;
    app.next_check = Instant::now() + Duration::from_secs(60);
    let idle = frame(&mut app, &ctx, [920.0, 760.0]);
    let control = cards(&idle)[1];
    let text = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if control.contains(t.pos) => {
                    Some((t.pos, t.galley.job.text.clone(), t.galley.size()))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let original = text(&idle);
    let (_tx, rx) = mpsc::channel();
    app.receiver = Some(rx);
    let querying = frame(&mut app, &ctx, [920.0, 760.0]);
    assert_eq!(
        original,
        text(&querying),
        "checking must not replace guard text or reflow controls"
    );
    app.receiver = None;
    app.next_check = Instant::now() + Duration::from_secs(10);
    assert_eq!(original, text(&frame(&mut app, &ctx, [920.0, 760.0])));
}

#[test]
fn refreshing_preserves_control_button_appearance() {
    let ctx = context();
    let mut app = App::empty();
    app.username = "test-account".into();
    app.password = Zeroizing::new("test-password".into());
    app.status.state = State::Online;
    let idle = frame(&mut app, &ctx, [920.0, 760.0]);
    let control = cards(&idle)[1];
    let button_fills = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if control.contains(r.rect.center())
                        && (r.rect.height() - 36.0).abs() < 1.0 =>
                {
                    Some((r.rect, r.fill, r.stroke))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let original = button_fills(&idle);
    assert_eq!(original.len(), 2);
    let (_tx, rx) = mpsc::channel();
    app.receiver = Some(rx);
    let querying = frame(&mut app, &ctx, [920.0, 760.0]);
    assert_eq!(
        original,
        button_fills(&querying),
        "refresh must not flash the connection buttons"
    );
    app.receiver = None;
    assert_eq!(
        original,
        button_fills(&frame(&mut app, &ctx, [920.0, 760.0]))
    );
}
