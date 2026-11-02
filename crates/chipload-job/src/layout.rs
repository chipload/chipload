// SPDX-License-Identifier: GPL-3.0-or-later
//! The window's panel layout, kept for the user in `layout.json` in the settings folder
//! (docs/specs/dockable-panels.md §5). Described in Chipload's own terms, a tree of splits
//! and groups of tabs, so that the format is Chipload's to version whatever the docking
//! crate does. A persisted format of its own, versioned as the tool library is
//! (`LibraryFile`).

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::library::{LibraryError, LibraryFile, load_library, save_library};

/// The layout file's name in the settings folder.
pub const LAYOUT_FILE: &str = "layout.json";
/// The layout format's version.
pub const LAYOUT_VERSION: i64 = 1;

/// A panel the user arranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Panel {
    Browser,
    Properties,
    Shapes,
    Report,
}

/// A part of the window: the canvas, a group of tabs, or two parts side by side or one
/// above the other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LayoutNode {
    /// The canvas, alone.
    Canvas,
    /// Panels as tabs of one group: `front` is the one shown (an index in `panels`),
    /// `folded` a group folded to its tab bar.
    Tabs {
        panels: Vec<Panel>,
        #[serde(default)]
        front: usize,
        #[serde(default)]
        folded: bool,
    },
    /// `left` takes `share` of the width (more than 0, less than 1), `right` the rest.
    SideBySide {
        share: f64,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
    /// `top` takes `share` of the height (more than 0, less than 1), `bottom` the rest.
    Stacked {
        share: f64,
        top: Box<LayoutNode>,
        bottom: Box<LayoutNode>,
    },
}

/// The window's layout. A panel not in it is closed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowLayout {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    pub root: LayoutNode,
}

impl Default for WindowLayout {
    /// The layout a new user starts with, giving today's widths in a 1200 × 800 window: the
    /// Browser on the left, Properties and Shapes as tabs on the right, the Report folded
    /// along the bottom.
    fn default() -> Self {
        let tabs = |panels: Vec<Panel>, folded: bool| LayoutNode::Tabs {
            panels,
            front: 0,
            folded,
        };
        WindowLayout {
            schema_version: LAYOUT_VERSION,
            written_by: None,
            root: LayoutNode::Stacked {
                share: 0.82,
                top: Box::new(LayoutNode::SideBySide {
                    share: 0.28,
                    left: Box::new(tabs(vec![Panel::Browser], false)),
                    right: Box::new(LayoutNode::SideBySide {
                        share: 0.49,
                        left: Box::new(LayoutNode::Canvas),
                        right: Box::new(tabs(vec![Panel::Properties, Panel::Shapes], false)),
                    }),
                }),
                bottom: Box::new(tabs(vec![Panel::Report], true)),
            },
        }
    }
}

impl WindowLayout {
    /// What is wrong with the layout, as "where: message" in English: the canvas exactly
    /// once, each panel at most once, no empty group, a front tab that exists, shares
    /// strictly between 0 and 1.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen = BTreeSet::new();
        let mut canvases = 0;
        node_problems(&self.root, "root", &mut seen, &mut canvases, &mut problems);
        if canvases != 1 {
            problems.push(format!(
                "root: the canvas is in it {canvases} times, not once"
            ));
        }
        problems
    }
}

fn node_problems(
    node: &LayoutNode,
    at: &str,
    seen: &mut BTreeSet<Panel>,
    canvases: &mut usize,
    problems: &mut Vec<String>,
) {
    let share_problem = |share: f64| !(share.is_finite() && share > 0.0 && share < 1.0);
    match node {
        LayoutNode::Canvas => *canvases += 1,
        LayoutNode::Tabs { panels, front, .. } => {
            if panels.is_empty() {
                problems.push(format!("{at}.tabs: a group holds at least one panel"));
            }
            if *front >= panels.len().max(1) {
                problems.push(format!("{at}.tabs.front: {front} is not one of its panels"));
            }
            for panel in panels {
                if !seen.insert(*panel) {
                    problems.push(format!("{at}.tabs: {panel:?} is in the layout twice"));
                }
            }
        }
        LayoutNode::SideBySide { share, left, right } => {
            if share_problem(*share) {
                problems.push(format!(
                    "{at}.side_by_side.share: {share} is not between 0 and 1"
                ));
            }
            node_problems(
                left,
                &format!("{at}.side_by_side.left"),
                seen,
                canvases,
                problems,
            );
            node_problems(
                right,
                &format!("{at}.side_by_side.right"),
                seen,
                canvases,
                problems,
            );
        }
        LayoutNode::Stacked { share, top, bottom } => {
            if share_problem(*share) {
                problems.push(format!(
                    "{at}.stacked.share: {share} is not between 0 and 1"
                ));
            }
            node_problems(top, &format!("{at}.stacked.top"), seen, canvases, problems);
            node_problems(
                bottom,
                &format!("{at}.stacked.bottom"),
                seen,
                canvases,
                problems,
            );
        }
    }
}

impl LibraryFile for WindowLayout {
    const VERSION: i64 = LAYOUT_VERSION;
    fn validate(&self) -> Vec<String> {
        self.problems()
    }
    fn stamp(&mut self) {
        self.schema_version = LAYOUT_VERSION;
        self.written_by = Some(crate::library::written_by());
    }
}

/// Load the layout: the default layout when there is no file yet. A newer version or a
/// file that can't be read is an error, for the window to fall back on the default layout
/// without overwriting the file.
pub fn load_layout(path: &Path) -> Result<WindowLayout, LibraryError> {
    load_library(path)
}

/// Save the layout: validated, atomic, a file it replaces kept as `.bak`.
pub fn save_layout(layout: &WindowLayout, path: &Path) -> Result<(), LibraryError> {
    save_library(layout, path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    fn corpus(rel: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/layout")
            .join(rel)
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chipload-layout-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(LAYOUT_FILE)
    }

    #[test]
    fn the_default_layout_is_valid_and_no_file_gives_it() {
        assert!(WindowLayout::default().problems().is_empty());
        let path = temp("missing");
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_layout(&path).unwrap(), WindowLayout::default());
    }

    #[test]
    fn the_frozen_v1_sample_loads_and_saves_back_the_same() {
        let layout = load_layout(&corpus("v1/layout.json")).unwrap();
        let LayoutNode::Stacked { bottom, .. } = &layout.root else {
            panic!("{:?}", layout.root);
        };
        assert_eq!(
            **bottom,
            LayoutNode::Tabs {
                panels: vec![Panel::Report],
                front: 0,
                folded: true
            }
        );
        let path = temp("round");
        save_layout(&layout, &path).unwrap();
        let again = load_layout(&path).unwrap();
        assert_eq!(again.root, layout.root);
    }

    #[test]
    fn a_newer_or_broken_layout_is_refused() {
        let path = temp("refused");
        std::fs::write(&path, r#"{"schema_version": 99, "root": "canvas"}"#).unwrap();
        assert!(matches!(
            load_layout(&path),
            Err(LibraryError::UnsupportedVersion { .. })
        ));
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load_layout(&path).is_err());
    }

    #[test]
    fn a_layout_has_one_canvas_each_panel_once_and_sound_shares() {
        let tabs = |panels: Vec<Panel>| LayoutNode::Tabs {
            panels,
            front: 0,
            folded: false,
        };
        let bad = |root: LayoutNode, what: &str| {
            let layout = WindowLayout {
                root,
                ..WindowLayout::default()
            };
            let problems = layout.problems().join("; ");
            assert!(problems.contains(what), "{problems}");
        };
        bad(tabs(vec![Panel::Browser]), "the canvas is in it 0 times");
        bad(
            LayoutNode::SideBySide {
                share: 1.0,
                left: Box::new(LayoutNode::Canvas),
                right: Box::new(tabs(vec![Panel::Shapes, Panel::Shapes])),
            },
            "share: 1 is not between 0 and 1",
        );
        bad(
            LayoutNode::Stacked {
                share: 0.5,
                top: Box::new(LayoutNode::Canvas),
                bottom: Box::new(tabs(vec![Panel::Shapes, Panel::Shapes])),
            },
            "Shapes is in the layout twice",
        );
        bad(
            LayoutNode::Stacked {
                share: 0.5,
                top: Box::new(LayoutNode::Canvas),
                bottom: Box::new(LayoutNode::Tabs {
                    panels: vec![],
                    front: 0,
                    folded: false,
                }),
            },
            "at least one panel",
        );
    }
}
