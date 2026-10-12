use super::*;

fn commands(
    desktop: &mut Desktop,
    mut action: impl FnMut(&mut Desktop, &egui::Context),
) -> Vec<egui::ViewportCommand> {
    let ctx = egui::Context::default();
    let out = ctx.run(egui::RawInput::default(), |ctx| action(desktop, ctx));
    out.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .clone()
}

#[test]
fn closing_hides_window_only_when_tray_is_available() {
    let mut desktop = Desktop::default();
    let out = commands(&mut desktop, |desktop, ctx| {
        desktop.close_requested(ctx, true)
    });
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose))
    );
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::Visible(false)))
    );
    assert!(desktop.hidden);
    assert!(!desktop.exiting);

    let mut unavailable = Desktop::default();
    let out = commands(&mut unavailable, |desktop, ctx| {
        desktop.close_requested(ctx, false)
    });
    assert!(out.is_empty());
    assert!(!unavailable.hidden);
    assert!(unavailable.exiting);
}

#[test]
fn first_use_notice_stays_visible_when_close_is_clicked() {
    let mut desktop = Desktop {
        show_notice: true,
        ..Default::default()
    };
    let out = commands(&mut desktop, |desktop, ctx| {
        desktop.close_requested(ctx, true)
    });
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose))
    );
    assert!(!desktop.hidden);
    assert!(!desktop.exiting);
}

#[test]
fn restore_and_explicit_exit_ignore_the_hide_preference() {
    let mut desktop = Desktop {
        hidden: true,
        ..Default::default()
    };
    let out = commands(&mut desktop, |desktop, ctx| {
        desktop.command(Command::Show, ctx)
    });
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::Visible(true)))
    );
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::Minimized(false)))
    );
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::Focus))
    );
    assert!(!desktop.hidden);
    let out = commands(&mut desktop, |desktop, ctx| {
        desktop.command(Command::Exit, ctx);
        desktop.close_requested(ctx, true);
    });
    assert!(desktop.exiting);
    assert!(
        out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::Close))
    );
    assert!(
        !out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose))
    );
}

#[test]
fn exit_setting_does_not_hide_the_window() {
    let mut desktop = Desktop::default();
    desktop.preferences.close = CloseBehavior::Exit;
    let out = commands(&mut desktop, |desktop, ctx| {
        desktop.close_requested(ctx, true)
    });
    assert!(desktop.exiting);
    assert!(!desktop.hidden);
    assert!(
        !out.iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose))
    );
}
