# Spec: dockable panels

Status: **agreed 2026-11-02**, decisions 1-7 as recommended (the user: "Sounds good, go
ahead"). Built 2026-11-02 (steps 1-3); ADR 0028.

As built (step 1):
- **The Report's summary is on its own tab, not in the status bar** (decision 3). The status
  bar's left side already holds hints and status lines.
  - Folded, the Report shows only its tab bar, as the old one-line bar did, and its tab
    reads "Report ⚠ 2" when it holds warnings.
  - The headline is the first line inside the panel.
- **The canvas is fixed as follows.** Its leaf's tab bar is hidden, and egui_dock's button
  to show it again and its drag margin are styled to nothing. A panel dropped into the
  canvas's middle is moved beside it.
- **The dock's own texts are not shown at all** (no tab context menus, no floating windows),
  so there was nothing of egui_dock's to translate.
- **The tabs are named for accessibility** in `on_tab_button`: egui_dock's tabs aren't by
  themselves, and the window tests find a panel by its title that way.
- **Shares, not pixels.** The default layout gives today's widths at 1200 × 800, and the
  panels grow with the window.

As built (step 2):
- **`chipload_job::layout`** holds the format (`WindowLayout`, `LayoutNode`, `Panel`), read
  and written through `LibraryFile` like the tool library.
  - The default layout is the format's `Default`, and the window builds its dock from it,
    so the default is defined once.
  - A missing file gives the default.
- **The window writes the layout** when it differs from the one last read or written, once
  no mouse button is down.
  - While P hides the panels, the layout written is the one P brings back.
  - Shares are kept to 1/1000.
  - A write that fails says so in the status bar, and isn't tried again until the layout
    changes.
- **The frozen sample** is `tests/corpus/layout/v1/layout.json`.


The user (2026-11-02), about the shape library tab: "In some UI this is user configurable
by dragging a panel onto another to create tabs and dragging below to stack. Is this a
feature in egui? I don't want a hack if we can avoid it." Then: "I think it is called
dockable panels." Kept apart from the window module cleanup, which came first and made
each panel's contents drawable on its own (`ChiploadApp::panel_contents`, layout.rs).

## 1. What a user can do

- **Arrange the panels.** Drag a panel's tab:
  - onto another panel's tab bar, and they become tabs of one panel;
  - onto a panel's edge, and it splits that panel: above, below, left or right;
  - onto the canvas's edge, and it becomes a panel along that side of the window.
- **Resize** by dragging the divider between two panels, as now.
- **Fold a panel** to its tab bar with its fold button, and click it to open it again.
- **Close a panel** with its ×. View ▸ Panels ▸ shows a check for each panel, to open it
  again.
- **Hide every panel with P**, to give the canvas the whole window, and bring them back the
  same way. P folds both side panels today.
- **View ▸ Reset layout** puts the panels back as they are by default (§3).
- **The arrangement is remembered** for the next start, for this user (§5). It is the same
  for every project.

The panels are the **Browser**, **Properties**, **Shapes** (the shape library) and
**Report** (decision 3). The canvas stays where it is: it has no tab, and it can't be
closed, moved or made a tab of something else.

## 2. Which crate

egui has no docking of its own. Two crates do it on egui 0.36, the version Chipload uses.
I read the source of both, fetched from crates.io.

| | egui_dock 0.21.1 (2026-08-06) | egui_tiles 0.17.1 (2026-08-18) |
|---|---|---|
| Licence | MIT | MIT OR Apache-2.0 |
| Its dependencies | duplicate, paste, thiserror (all MIT OR Apache-2.0) | ahash, itertools, log (all MIT OR Apache-2.0) |
| Tabs, splits, drag to dock | yes | yes |
| Splits | in two (repeatable) | rows, columns and grids |
| Close button per tab | yes, with `is_closeable` per tab | do it yourself |
| Fold a panel to its tab bar | yes (`show_leaf_collapse_buttons`) | no |
| Tab bar hidden for one panel | yes | through its `Behavior` |
| Scroll areas per tab | built in (`scroll_bars`) | do it yourself |
| Floating windows | yes, can be turned off per tab (`allowed_in_windows`) | no |
| Its own texts (context menu) | translatable (`translations`) | none |
| Saving the layout | serde | serde |
| Maintained by | its community | Rerun, egui's authors |

Neither is a hack: each is a crate used through its API (CLAUDE.md, dependencies). Both
licences are on the allowed list.

One note on egui_dock: its dependency `paste` is marked unmaintained in the RustSec advisory
database (RUSTSEC-2024-0436). It is a build-time macro with no known flaw. `cargo deny` here
checks licences, bans and sources, not advisories, so it doesn't stop the build. The ADR
records the advisory.

## 3. The default layout

The same as today:
- the Browser on the left;
- the canvas in the middle;
- Properties and Shapes as two tabs of one panel on the right, Properties in front;
- the Report along the bottom (decision 3).

The widths are today's: the Browser 340, the right panel 440.

## 4. How it is built

- **One dock area** fills the window below the ribbon and above the status bar.
  - Its tabs are a `Pane` enum: `Canvas`, `Browser`, `Properties`, `Shapes`, `Report`.
  - Its `TabViewer` calls `panel_contents(ui, kind)` for a panel, and `canvas(ui)` for the
    canvas.
  - The canvas's tab can't be closed, dragged or put in a window, and its tab bar is
    hidden.
  - No floating windows (decision 4).
- **What the window does now with panels moves to the dock:**
  - `Panels` (layout.rs) keeps the dock's state in place of `browser_open`,
    `properties_open` and `right_tab`.
  - Code that shows a panel today (for example "back to Properties" when the Browser picks
    something, shape-library.md decision 10) asks the dock to bring that tab to the front
    instead.
  - `side_panels` and the folded strips go.
- **The shape library's drop target** today is the Shapes tab's name in the right panel.
  It becomes the Shapes panel's tab, wherever that is.
- **The dock's own texts** are in the app's language, in all six languages: its context
  menu's Close, Fold and Hide tab bar, and the tooltips on its buttons.
- **Narrow panels:** the forms already lay out down to `PROPERTIES_MIN_WIDTH` (250).
  - egui_dock has no least width per tab, so a panel can be dragged narrower than that.
    The forms then scroll sideways, as they already do in a narrow window.
  - The narrow-window test (`narrow_panel.rs`) keeps checking the forms at that width.

## 5. Saving the layout

The layout is kept in its own file, `layout.json`, in the user's settings folder. It is
not kept in `settings.json`, for two reasons:
- `settings.json` refuses unknown fields, so an older Chipload would refuse the whole file
  if the layout were added to it;
- a broken layout file must not cost the user their settings.

The file is a persisted format like the others:
- **Fields:** `schema_version` 1 and `written_by`, as the other persisted files have.
- **The layout itself is described in Chipload's own terms**, not in egui_dock's saved
  form (decision 6): the tree of splits (direction and share) and tab groups (their panels
  in order, the one in front, folded or not), plus which panels are closed.
- **Reading:** goes through one `load_layout`.
  - An older version is migrated.
  - A newer version, or a file that can't be read, gives a typed error.
  - On that error the window uses the default layout and says so once in the status bar.
  - It doesn't overwrite the file until the user changes the layout, so a newer Chipload's
    file survives a run of an older one.
- **Writing:** when the layout changes, once the drag has ended.

## 6. Tests

- **The default layout:** the panels and the canvas are where §3 says. The window tests that
  find a panel by its title keep working.
- **Arranging through the dock's API:**
  - Shapes moved below Properties: both show at once.
  - The Browser closed, then opened again from View ▸ Panels.
  - P hides every panel and brings them back as they were.
- **Saving:**
  - A layout written and read back is the same.
  - A newer version, or a broken file, gives the default layout, and the file stays as it
    was.
  - A frozen sample of version 1 is kept in `tests/corpus/layout/v1/`, as for the other
    persisted formats.
- **Pictures:** the window pictures and the guide's screenshots are taken again with the
  default layout, which should look as today apart from the tab bars.

## 7. Order of work

1. **The dependency and the dock**, with the default layout, nothing saved. View ▸ Panels,
   Reset layout and P. The ADR goes in this commit; THIRD-PARTY-LICENSES.txt is
   regenerated.
2. **`layout.json`:** saving, reading and its sample.
3. **The guide:** a page, *Arranging the window*, and new screenshots.

## 8. Decisions

1. **egui_dock 0.21.1.** *Recommended.* It has what this needs built in: close buttons,
   folding a panel, a hidden tab bar for the canvas, scroll areas, and translatable texts.
   *Instead:* egui_tiles 0.17.1, by egui's own authors, with rows, columns and grids. It
   would need close buttons, folding and scroll areas written here. Either way the dock
   crate follows egui's versions: egui_tiles 0.18 already needs egui 0.37.
2. **The canvas is fixed.** *Recommended:* it is the window's work surface, so it can't be
   closed, moved or tabbed, and its tab bar is hidden. *Instead:* the canvas as one more
   panel. That makes it possible to lose it.
3. **The Report becomes a dockable panel.** *Recommended*, so that every panel behaves the
   same way.
   - Its one-line summary, the first line and the number of further warnings, moves to
     the status bar, which is always shown.
   - The panel still opens by itself on warnings and errors, as now.

   *Instead:* the Report stays as it is, a bar along the bottom outside the dock. Less
   changes, but there are two kinds of panel.
4. **No floating windows.** *Recommended:* separate windows behave differently on Windows,
   Linux and WSLg, and the panels all fit in one window. *Instead:* allow dragging a panel
   out into its own window inside the main one (egui_dock can).
5. **P hides every panel.** *Recommended:* one key for "the canvas, as large as it can be",
   and back as it was. Folding single panels is done with their fold buttons.
   *Instead:* keep P as "fold both side panels". With panels anywhere, there is no "side"
   any more.
6. **The layout is saved in Chipload's own terms** (§5). *Recommended:* the file's format is
   Chipload's to version and migrate, whatever egui_dock does to its saved form between
   releases. That costs a small conversion each way. *Instead:* save egui_dock's own serde
   form inside the versioned wrapper. That is less code, but a crate upgrade can make the
   saved layout unreadable, so a version change would mean falling back to the default.
7. **One layout for the user**, in the settings folder. *Recommended:* the window is
   arranged to the user's liking, not the project's. *Instead:* one per project, kept in
   the project file. That would change the project format for something that isn't about
   machining.
