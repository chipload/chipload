# 0028: Dockable panels with egui_dock

Status: accepted (2026-11-02). Spec: docs/specs/dockable-panels.md (agreed 2026-11-02,
decisions 1-7 as recommended).

## Context
The user asked for panels arranged by hand: dragged onto another panel for tabs, or
below it to stack, "no hack". egui has no docking of its own. Two maintained crates do it
on egui 0.36:
- egui_dock 0.21.1 (MIT);
- egui_tiles 0.17.1 (MIT OR Apache-2.0, by egui's authors).

Both were read from their source before choosing. Before this, the window module cleanup
made each panel's contents drawable apart from where the panel sits
(`ChiploadApp::panel_contents`).

## Decision
- **egui_dock 0.21.1**, without its `serde` feature. It has built in what Chipload needs:
  - a close button per tab, refused for the canvas;
  - folding a panel to its tab bar;
  - a leaf without a tab bar, for the canvas;
  - per-tab style overrides;
  - floating windows that can be turned off.

  egui_tiles would need these written here.
- **One dock area** under the ribbon, of `Pane`s: the canvas, the Browser, Properties,
  Shapes and the Report.
  - The canvas is fixed. Its leaf has no tab bar, and the bar's show button and drag margin
    are styled to nothing.
  - A panel dropped into the canvas's middle is moved beside it (`Panels::settle`).
  - No floating windows; no tab context menus (View ▸ Panels does what they would).
- **Folding a leaf** keeps its splits' counts of folded leaves right through egui_dock's
  public `Node` API (`set_folded`), since its own update for this is crate-private.
- **Changes asked for while the dock is drawn** (the Report opening on a warning, a save to
  the shape library) wait in `Panels::pending` and are made once it is drawn. The dock's
  state is taken out of the app while it is drawn, so that the panels can draw the app.
- **Each tab is named for accessibility** in `on_tab_button`, as egui's own widgets are;
  egui_dock's tabs aren't by themselves.
- New crates in the lock file: egui_dock (MIT), and duplicate, paste and
  proc-macro2-diagnostics (MIT OR Apache-2.0).
  - `paste` is marked unmaintained in the RustSec advisory database
    (RUSTSEC-2024-0436). It is a build-time macro with no known flaw.
  - `cargo deny` here checks licences, bans and sources, not advisories.

## Consequences
- The default layout has today's widths at the default window size. Split shares are
  fractions, so the panels now grow with the window, where they used to keep their width.
- The Report is a panel like the others. Folded, its tab says how many warnings it holds.
  The headline is the first line inside it.
- P hides every panel and brings them back as they were, instead of folding the two side
  panels.
- The layout isn't saved yet: step 2 of the spec adds `layout.json` in Chipload's own
  terms.
- egui_dock follows egui's versions. An egui upgrade waits for a matching egui_dock.
