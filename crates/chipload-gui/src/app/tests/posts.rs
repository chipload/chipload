// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_user_post_is_chosen_copied_in_and_updated() {
    let (mut app, dir) = with_settings("user-post");
    let posts = dir.join("posts");
    std::fs::create_dir_all(&posts).unwrap();
    let config = "schema_version = 1\nname = \"Shop\"\ntemplate = \"shop.nc.j2\"\n\
                  spindle_dwell_s = 2\n";
    std::fs::write(posts.join("shop.toml"), config).unwrap();
    let template = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../chipload-post/templates/grbl.nc.j2"),
    )
    .unwrap();
    std::fs::write(posts.join("shop.nc.j2"), &template).unwrap();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert!(app.choose_project_post("shop"));
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert_eq!(job.machine.post, "shop");
    assert!(job.machine.custom_post.is_some());
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let state = |app: &ChiploadApp| {
        let job = app.edit.as_ref().unwrap().to_job().unwrap();
        chipload_pipeline::posts::copy_state(&job, Some(&posts))
    };
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Same);
    std::fs::write(
        posts.join("shop.nc.j2"),
        template.replace("Chipload", "Shop"),
    )
    .unwrap();
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Changed);
    assert!(app.choose_project_post("shop"), "Update copies it again");
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Same);
    assert!(!app.choose_project_post("nope"));
    assert!(app.choose_project_post("grbl"));
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert!(job.machine.custom_post.is_none());
}
