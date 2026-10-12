use super::*;

#[path = "refresh.rs"]
mod refresh;

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    App::configure_context(&ctx);
    ctx
}

fn frame(app: &mut App, ctx: &egui::Context, size: [f32; 2]) -> egui::FullOutput {
    input_frame(app, ctx, size, Vec::new())
}

fn input_frame(
    app: &mut App,
    ctx: &egui::Context,
    size: [f32; 2],
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size[0], size[1]),
            )),
            ..Default::default()
        },
        |ctx| app.draw(ctx),
    )
}

fn cards(output: &egui::FullOutput) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == theme::CARD
                    && rect.stroke.color == theme::BORDER
                    && rect.corner_radius.nw == 8
                    && rect.rect.width() > 200.0 =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn refresh_button_stays_at_status_card_right_edge() {
    for size in [[920.0, 760.0], [540.0, 1100.0], [1240.0, 940.0]] {
        let ctx = context();
        let mut app = App::empty();
        for busy in [false, true, false] {
            app.receiver = if busy { Some(mpsc::channel().1) } else { None };
            let output = frame(&mut app, &ctx, size);
            let status = cards(&output)[0];
            let button = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if status.contains(r.rect.center())
                            && (r.rect.width() - 124.0).abs() < 1.0
                            && (r.rect.height() - 36.0).abs() < 1.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("refresh button");
            assert!(
                (status.right() - button.right() - 21.0).abs() < 1.0,
                "size={size:?}, busy={busy}: {status:?}, {button:?}"
            );
        }
    }
}

#[test]
fn default_layout_has_equal_height_40_60_columns_and_stable_busy_state() {
    let ctx = context();
    let mut app = App::empty();
    let idle = cards(&frame(&mut app, &ctx, [920.0, 760.0]));
    assert_eq!(idle.len(), 4, "cards: {idle:?}");
    assert!(
        (idle[1].height() - idle[2].height()).abs() < 1.0,
        "{idle:?}"
    );
    assert!((idle[1].height() - 244.0).abs() < 1.0, "{idle:?}");
    assert!(
        (idle[1].width() / (idle[1].width() + idle[2].width()) - 0.4).abs() < 0.02,
        "{idle:?}"
    );
    assert!(
        (idle[2].left() - idle[1].right() - 20.0).abs() < 1.0,
        "{idle:?}"
    );
    let (_tx, rx) = mpsc::channel();
    app.receiver = Some(rx);
    let busy = cards(&frame(&mut app, &ctx, [920.0, 760.0]));
    assert!((idle[0].height() - busy[0].height()).abs() < 1.0);
}

#[test]
fn narrow_layout_stacks_cards_without_overlapping() {
    let ctx = context();
    let mut app = App::empty();
    let bounds = cards(&frame(&mut app, &ctx, [540.0, 1100.0]));
    assert_eq!(bounds.len(), 4, "{bounds:?}");
    assert!(bounds[2].top() >= bounds[1].bottom() + 19.0, "{bounds:?}");
    assert!((bounds[1].width() - bounds[2].width()).abs() < 1.0);
    assert!(
        bounds
            .iter()
            .all(|r| r.left() >= 27.0 && r.right() <= 513.0),
        "{bounds:?}"
    );
}

#[test]
fn input_borders_and_icon_centers_align_at_multiple_sizes() {
    for size in [[920.0, 760.0], [540.0, 1100.0], [1240.0, 940.0]] {
        let ctx = context();
        let mut app = App::empty();
        let output = frame(&mut app, &ctx, size);
        let inputs: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.fill == theme::INPUT
                        && r.stroke.color == theme::BORDER
                        && r.corner_radius.nw == 6 =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(inputs.len(), 2, "{inputs:?}");
        assert!((inputs[0].left() - inputs[1].left()).abs() < 1.0);
        assert!((inputs[0].right() - inputs[1].right()).abs() < 1.0);
        let arrow = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Path(p) if p.points.len() == 3 && inputs[0].contains(p.points[1]) => {
                    Some(p.points[1].x)
                }
                _ => None,
            })
            .unwrap();
        let eye = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Circle(c)
                    if (c.radius - 3.0).abs() < 0.1 && inputs[1].contains(c.center) =>
                {
                    Some(c.center.x)
                }
                _ => None,
            })
            .unwrap();
        assert!((arrow - eye).abs() < 1.0, "{arrow} != {eye}");
    }
}

#[test]
fn expanded_logs_remain_below_controls_and_use_available_height() {
    for size in [[920.0, 760.0], [1240.0, 940.0], [540.0, 1100.0]] {
        let ctx = context();
        let mut app = App::empty();
        app.show_logs = true;
        for _ in 0..40 {
            app.log("test-only diagnostic response with a deliberately long message that must wrap within the log content area");
        }
        let bounds = cards(&frame(&mut app, &ctx, size));
        assert_eq!(bounds.len(), 4, "{bounds:?}");
        let bottom = bounds[1].bottom().max(bounds[2].bottom());
        assert!(bounds[3].top() >= bottom + 19.0, "{bounds:?}");
        assert!(bounds[3].right() <= size[0] - 27.0, "{bounds:?}");
        assert!(
            (bounds[3].bottom() - (size[1] - 28.0)).abs() < 6.0,
            "unfilled or overflowing logs: {bounds:?}"
        );
    }
}

#[test]
fn display_scaling_does_not_clip_or_overlap_cards() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let ctx = context();
        let mut app = App::empty();
        app.show_logs = true;
        for size in [[920.0, 760.0], [540.0, 1100.0]] {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(size[0], size[1]),
                )),
                ..Default::default()
            };
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .native_pixels_per_point = Some(scale);
            let output = ctx.run(input, |ctx| app.draw(ctx));
            let bounds = cards(&output);
            assert_eq!(bounds.len(), 4, "scale={scale}: {bounds:?}");
            assert!(
                bounds
                    .iter()
                    .all(|rect| rect.left() >= 27.0 && rect.right() <= size[0] - 27.0),
                "scale={scale}: {bounds:?}"
            );
            assert!(
                bounds[3].top() >= bounds[1].bottom().max(bounds[2].bottom()) + 19.0,
                "scale={scale}: {bounds:?}"
            );
        }
    }
}

#[test]
fn opening_large_log_history_creates_only_visible_text_widgets() {
    let ctx = context();
    let mut app = App::empty();
    for index in 0..5000 {
        app.log(format!(
            "virtualized-diagnostic-{index}: original raw response preserved"
        ));
    }
    frame(&mut app, &ctx, [920.0, 760.0]);
    app.show_logs = true;
    let output = frame(&mut app, &ctx, [920.0, 760.0]);
    let visible = output
        .shapes
        .iter()
        .filter(|s| match &s.shape {
            egui::Shape::Text(text) => text.galley.job.text.contains("virtualized-diagnostic-"),
            _ => false,
        })
        .count();
    assert!(
        visible > 0 && visible < 30,
        "rendered {visible} text widgets for 5000 logs"
    );
    assert_eq!(app.logs.len(), 5000);
}
