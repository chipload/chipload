// SPDX-License-Identifier: GPL-3.0-or-later
//! The planner's errors are messages; in English they read as they always have.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_i18n::{Digits, Message, Value};
use chipload_toolpath::ToolpathError;

#[test]
fn errors_read_as_before_in_english() {
    let tabs = |fits: u64| {
        ToolpathError::Geometry(
            Message::new("toolpath-error-tabs-fit")
                .arg("count", 4)
                .arg("width", Value::Length(6.0, Digits::Fixed(3)))
                .arg("length", Value::Length(40.0, Digits::Fixed(1)))
                .arg("fits", fits),
        )
        .to_string()
    };
    assert_eq!(
        tabs(0),
        "4 tabs of 6.000 mm don't fit on a 40.0 mm path (not even one fits)"
    );
    assert_eq!(
        tabs(2),
        "4 tabs of 6.000 mm don't fit on a 40.0 mm path (at most 2 fit)"
    );
    let rings = Message::new("toolpath-error-too-many-rings")
        .arg("contours", "#1, #2")
        .arg("most", 20_000);
    assert_eq!(
        rings.english(),
        "contours #1, #2: more than 20000 pocket rings"
    );
    let label = Message::new("job-operation-label").arg("number", 1);
    let context = Message::new("toolpath-error-label-contour")
        .arg("label", label)
        .arg("contour", 3);
    let err = ToolpathError::Geometry(Message::new("toolpath-error-joined-cross"))
        .with_context(context)
        .at(0);
    assert_eq!(
        err.to_string(),
        "Operation 1, contour #3: joined outlines cross each other"
    );
    let nothing =
        Message::new("toolpath-error-selects-nothing").arg("selector", Value::Text(String::new()));
    assert_eq!(nothing.english(), "selector  matches nothing");
    let failed = Message::new("toolpath-error-library-failed").arg(
        "what",
        Message::new("toolpath-error-medial-step").arg(
            "name",
            Message::new("toolpath-error-medial-name").arg("name", "vcarve"),
        ),
    );
    assert_eq!(
        failed.english(),
        "The V-carve centre line failed inside the geometry library"
    );
    assert_eq!(ToolpathError::Cancelled.to_string(), "Generation cancelled");
}
