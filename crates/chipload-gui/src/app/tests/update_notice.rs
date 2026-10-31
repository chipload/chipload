// SPDX-License-Identifier: GPL-3.0-or-later
//! The update notice in the window (docs/specs/update-notice.md §2, §8). No test reaches
//! the network: the request is replaced by one that gives a set answer.
use super::*;
use crate::updates::{Outcome, Version};

/// GitHub's answer for a release far newer than this one.
fn far_newer() -> Result<String, String> {
    Ok(r#"{"tag_name": "v99.0.0",
           "html_url": "https://github.com/chipload/chipload/releases/tag/v99.0.0"}"#
        .to_string())
}

fn offline() -> Result<String, String> {
    Err("no network".to_string())
}

fn settings(dir: &std::path::Path) -> chipload_job::library::AppSettings {
    chipload_job::library::load_settings(&chipload_pipeline::library::settings_path(dir)).unwrap()
}

/// Draw the status bar and the Help menu's items for one frame.
fn draw_status_and_help(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.status_bar(ui);
        app.help_menu(ui);
    });
    output.textures_delta.clear();
}

#[test]
fn the_question_is_asked_once_and_nothing_is_checked_before_an_answer() {
    // A folder left by an earlier run with the same process id holds its answer.
    let _ = std::fs::remove_dir_all(temp_dir("updates-question"));
    let (mut app, dir) = with_settings("updates-question");
    app.updates.fetch = far_newer;
    assert!(app.update_question_shown());
    app.poll_updates(&egui::Context::default());
    assert!(!app.updates.checking(), "no check before an answer");
    // Closed: gone for this run, asked again next start.
    app.updates.bar_closed = true;
    assert!(!app.update_question_shown());
    assert_eq!(settings(&dir).update_check, None);
    // No: kept, and never checked.
    let (mut app, dir) = with_settings("updates-question");
    app.updates.fetch = far_newer;
    app.answer_update_question(false);
    assert_eq!(settings(&dir).update_check, Some(false));
    app.poll_updates(&egui::Context::default());
    assert!(!app.updates.checking());
    let (again, _) = with_settings("updates-question");
    assert!(!again.update_question_shown(), "answered once");
}

#[test]
fn yes_checks_at_once_and_a_newer_version_shows_until_skipped() {
    let (mut app, dir) = with_settings("updates-newer");
    let _ = std::fs::remove_file(chipload_pipeline::library::settings_path(&dir));
    app.set_settings_dir(Some(dir.clone()));
    app.updates.fetch = far_newer;
    app.answer_update_question(true);
    let ctx = egui::Context::default();
    app.poll_updates(&ctx);
    assert!(app.updates.checking(), "the daily check, at once");
    assert!(settings(&dir).update_checked_at.is_some());
    let (outcome, manual) = app.updates.wait().unwrap();
    assert!(!manual);
    assert!(matches!(outcome, Outcome::Newer(_)));
    let release = app.updates.notice().cloned().unwrap();
    assert_eq!(release.version, Version::parse("99.0.0").unwrap());
    draw_status_and_help(&mut app);
    // The next start doesn't check again the same day.
    let (mut again, _) = with_settings("updates-newer");
    again.updates.fetch = far_newer;
    again.poll_updates(&ctx);
    assert!(!again.updates.checking(), "once a day");
    // Skipped: no notice, kept in the settings.
    app.skip_update(release.version);
    assert!(app.updates.notice().is_none());
    assert_eq!(settings(&dir).update_skipped.as_deref(), Some("99.0.0"));
    draw_status_and_help(&mut app);
}

#[test]
fn check_now_works_with_the_daily_check_off_and_says_why_it_failed() {
    let (mut app, _) = with_settings("updates-now");
    app.answer_update_question(false);
    app.updates.fetch = offline;
    app.check_updates_now(&egui::Context::default());
    let (outcome, manual) = app.updates.wait().unwrap();
    assert!(manual);
    assert_eq!(outcome, Outcome::Failed("no network".into()));
    // Check now always answers; the status line says so.
    app.updates.fetch = offline;
    app.check_updates_now(&egui::Context::default());
    while app.updates.checking() {
        app.poll_updates(&egui::Context::default());
        std::thread::yield_now();
    }
    let status = app.status.as_ref().map(|(text, _)| text.clone()).unwrap();
    assert!(status.contains("no network"), "{status}");
    // Its row in Preferences turns the daily check on.
    app.focus = Focus::Preferences;
    draw_properties(&mut app);
}
