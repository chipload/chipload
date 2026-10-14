# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

(nothing)

## State to remember (2026-10-12)

- **Release 0.13.0**: tag `v0.13.0` (14a1b71), published on GitHub with the Linux files
  (AppImage, CLI tar.gz) and, since 2026-10-12, the Windows files (GUI and CLI `.exe`), with
  `SHA256SUMS.txt` covering all four. The Windows GUI hasn't been started on Windows yet:
  try it. Old drafts (0.3.0–0.10.1) were deleted; their tags remain.
- **No GitHub Actions minutes left**: the release workflow can't run. Linux release files
  are built locally with `packaging/linux/build-release-local.sh <version> [out]` (podman,
  Ubuntu 22.04 container, tests first, the same steps as the workflow), then uploaded with
  `gh release create v<version> … --draft --verify-tag`. The `gh` token now has Contents
  read/write on `gntech/chipload` (needed for releases). Remember the usual release steps
  first (docs/releasing.md: version bump, internal pins, `cargo about generate`).
- **Windows builds** (local, no Actions): Rust (msvc) and the Build Tools are installed on
  the Windows side, checkout `C:\code\chipload`; the steps are in docs/releasing.md
  (*Without GitHub Actions*). 0.13.0 was built and uploaded that way.
- **The repository is private**: testers can't download releases or open issues without
  being added as collaborators. When it goes public, publish the user guide: the steps are
  at the end of docs/releasing.md (the publish script's dry run passes, 2026-10-13). Decide: make it public, add testers, or another way to
  collect reports (the 0.13.0 notes ask for issues).
- **Branches**: `release-0.12` (the 0.12.1 fix branch, kept for
  history). Tag `adaptive-final` (local): main before the removal.
- **Waiting on the user**: real-controller tests of the LinuxCNC and Mach3/4 posts.
- **Property tests** (fixed 2026-10-13): release builds run them with a fixed seed
  (`packaging/test-release.sh`), so a release can't fail on an unseen case; everyday runs
  keep drawing new cases, and failures are now saved next to each test file (they weren't:
  proptest's default needs a lib.rs or main.rs) to commit with the fix. The
  `material_left_is_out_of_reach_and_complete` failure was found again (seed 59245510) and
  fixed: an island loop that touches itself came back from the offset library as open scraps.

## Next

- **Commercial-CAM features** (user, 2026-10-13: "1-5 and 7 we should add"), each with its
  own spec first, in this order:
  1. **Follow path** — done 2026-10-13 (docs/specs/follow-path.md, schema 17).
  2. **Finishing pass and onion skin** — done 2026-10-13 (docs/specs/finish-and-skin.md,
     optional profile fields, no version bump). Later: a separate finishing feed, and a
     finishing pass for pockets (walls and floor), each with its own spec.
  3. **Drilling and helical boring** — done 2026-10-13 (docs/specs/holes.md, schema 18).
     Later (§9 there): canned cycles G81/G83 as a post option, DXF points as hole centres,
     a clearance for a fit, dwell, chip-breaking pecks, V-bit chamfers, counterbores in one
     operation.
  4. **Dogbone and T-bone corners** — done 2026-10-13 (docs/specs/dogbones.md, optional
     `corners` field, no version bump). Later: a choice per corner on the canvas.
  5. **Surfacing / facing**: flatten the spoilboard or face the stock (raster or spiral).
     Skipped for now (user, 2026-10-13): after the simulation.
  6. **Material-removal simulation** — done 2026-10-13 (docs/specs/simulation.md). Later:
     comparing with the design (gouges), holders and flute length. Playback done
     2026-10-13 (docs/specs/playback.md); later: a G-code panel with the current line.

  The user will test features 1–4 and 6, and the playback slider, and give feedback
  (2026-10-13).

- **Trochoidal and adaptive clearing: removed 2026-10-13** (docs/decisions/0017-scope.md).
  The code is at the local tag `adaptive-final` (the work branches were deleted
  2026-10-13, with the user's agreement). Projects that use
  them load and are refused when generating ("switch the pocket to rings").
- **Lean plan** (approved 2026-10-13, ADR 0017), in this order, each with a spec first:
  1. Operation presets and layer rules: paused 2026-10-13 by the user, spec drafted
     (docs/specs/presets.md, decisions not reviewed).
  2. Rest machining — done 2026-10-13 (docs/specs/rest-machining.md). Later: rest
     profiles if missed; dogbones on a rest pocket.
  3. Scripting — done 2026-10-13 (docs/specs/scripting.md, docs/book/src/scripting.md). Batch
     runs: shell recipes instead of a command (decision 8).
  4. Feeds and speeds per material in the library, with a chip-load warning.
  5. Material left merged into the simulation — done 2026-10-13
     (docs/specs/material-left-merge.md).
  6. Fuzz the DXF importer — done 2026-10-13 (fuzz/README.md; one overflow found and fixed).
  Also: the drawing tools are frozen (ADR 0017); real-machine validation (the user).
- **General improvement pass** (proposed and approved 2026-10-12, in this order):
  1–2. Done (background generation and live preview, see Done).
  3–4. Done (recovery copies and the `.bak` on save, see Done).
  5. Done (user post-processors, LinuxCNC and Mach3/4 posts, see Done).
  6. Done (problems while editing, see Done).
  7–8. Done (cycle time estimate and the setup sheet, see Done).
  - Small items done (see Done). Later from them: copy imported shapes (as drawn
    polylines); a chip load range per tool in the library (amber warning, schema change).
  - Later from the same pass: a bug-report bundle, inch display and entry, operation
    presets (now in the lean plan; toolpath playback: done, see Done).
- **Chipload in the browser** (docs/specs/web.md): step 1 done 2026-10-13 (the libraries
  compile for wasm32, checked in CI). Step 2, the browser app (a Web Worker, upload and
  download, browser storage, a bundled font), after user testing; its decisions are
  proposals to review then.
- **Image fuzzing got slow** (2026-10-13): 38 inputs a second with arcs and centre lines,
  against thousands before; find which pictures are slow (centre lines on noise?) and
  whether a user could hit it.
- A light canvas for the light theme (the canvas is dark in both themes; its colours are
  tuned for dark). Needs a second canvas palette.

## Done

- **A clearer image window** (2026-10-14): the traced parts show near black on white, and
  outlines are no longer drawn over them, so the detail stays visible while setting the
  threshold; centre lines are still drawn, as they are the result.
- **The preview is of the active part** (2026-10-14, docs/specs/active-part-preview.md):
  with several parts, generating, simulating and playback cover the active part; All parts
  on the canvas bar shows every part; export always plans and writes every part.
- **V-carving a whole picture** (fix, 2026-10-14): the app took a selector of only a DXF
  or picture (`{"image": 1}`) for "no geometry"; one `Selector::is_empty` now decides
  everywhere (app, project rules, start points on placing). A new V-carve takes the
  project's V-bit, other kinds a non-V-bit.
- **Direct placement** (2026-10-14, docs/specs/direct-placement.md, schema 24): a DXF's
  properties are Position (its box's corner), Rotation (any angle) and Scale, each with a
  reset; pictures get Position, Rotation and Width, drawn turned on the canvas;
  `place --rotate` any angle, `place --at`, `image set --rotate`.
- **Use selection with a picture** (fix, 2026-10-14): traced contours picked on the canvas
  became shapes by id (refused) in *Use selection* and *Copy as selector*; now the picture,
  minus what isn't picked, as for a new operation.
- **This computer** (2026-10-14, docs/specs/this-computer.md): the Browser's top root holds
  Preferences (setup sheet, update by itself, theme), Tools, Machines and the
  post-processors with what each writes; `chipload post show`.
- **A linked machine** (2026-10-14, docs/specs/linked-machine.md): its machine values are
  read-only until *Edit here*, a row that differs has a revert button, and safe Z, work zero
  and max depth are job values that never count as differing.
- **Playback along long moves** (2026-10-14, docs/specs/playback.md §3): the stock is cut
  as the tool goes along a move, not at its end; the Library is at the top of the Browser.
- **The library in the Browser** (2026-10-14, docs/specs/library-in-browser.md, ADR 0018):
  Library > Tools and Machines above the project, edited in Properties (drafts until Save);
  the library windows are gone. Project tools and the machine link to their entry by `uid`
  (schema 23, tools/machines files 3) and show same, differs, missing or none; drag a library
  tool onto the project's Tools; `tool compare`, `machine compare`.
- **One place for what's shown; no Settings menu** (2026-10-14,
  docs/specs/view-and-settings.md): the canvas bar has labelled Fit, Top, 3D, Stock,
  Toolpath, Images and Grid; a LIBRARY ribbon group (Tools, Machines); the theme button in
  the top bar; the setup sheet in Job settings > Machine.
- **Unsaved changes** (0.17.4): Save, Don't save or Cancel, in a centred modal.
- **Leaving shapes out of a whole DXF or picture** (2026-10-13,
  docs/specs/leave-out-shapes.md): Ctrl+click on a contour of the selected source;
  `select.exclude`; left-out traced shapes found again on retrace, or a blocking problem.
- **Traced pictures fixes** (0.17.2): closed centre lines can be profiled; clicking a
  traced contour selects its picture.
- **Holding the part** (2026-10-13, docs/specs/holding.md): tabs and the onion skin as one
  choice (none, tabs, onion skin, both) in a Holding section; Finishing pass on its own.
- **A tidier profile form** (2026-10-13, docs/specs/profile-form.md): *Spiral ramp* and
  its angle right after the step down; the lead only with straight plunges (an existing
  spiral-plus-lead stays, shown); the profile's Entry and lead section is gone.
- **Tools, DXFs and pictures in Properties** (2026-10-13,
  docs/specs/properties-for-tools-and-dxfs.md): clicking *Tools* or a tool, a DXF (or its
  layer) or a picture in the Browser shows its properties; Job settings keeps stock,
  machine and post; the Place window is gone; the picture window keeps the trace
  settings only.
- **Parts** (2026-10-13, docs/specs/parts.md, schema 22): operations grouped into parts,
  cut part by part, each with its copies and on/off; one active part for new operations;
  copies may not cut into other parts; a G-code comment where each part starts; the
  `part` commands, `op part`, `copies --part`; parts on the setup sheet and in the report;
  the Browser's Parts folder with part properties. Golden v21_copies_by_part (frozen by
  0.16.0) and v22_two_parts. Later, if missed: grouping by tool across parts, dimming the
  other parts on the canvas, a "setup" level (stock and zero) above parts.
- **The Browser and a more professional layout** (2026-10-13,
  docs/specs/browser-and-layout.md, GUI only): the Browser (one tree: project, Job
  settings, Tools, Drawing, Operations; painted icons, eyes, ticks, right-click menus, drag
  reordering); Properties with the selection and the shape editor; a top bar and a ribbon
  (FILE, DRAW, OPERATION, VIEW, captions open the menus) in place of the menu bar and
  toolbar; the canvas with a grid on the snap steps (G, `show_grid` in settings), axis
  lines, a gradient and a light palette, a navigation bar and an orientation cube.
- **Several DXFs and pictures per project, the Drawing panel, hiding, hide the toolpath**
  (2026-10-13, docs/specs/drawing-sources.md, schema 21): `drawing.dxfs` with a placement
  each, `dxf add/list/remove`, `place --dxf`, `update --dxf`; selectors limited to a DXF or
  image, a note when a layer is in several; dropping DXFs and pictures adds them; the
  Drawing panel (was Shapes) with groups, Select, ⋯ (place, update, edit, remove) and
  hiding (still cut, noted on the card); View > Show toolpath. Golden v20_scaled_part,
  v21_two_dxfs. Later: drag a DXF on the canvas to move it.
- **Traced pictures with arcs, and centre lines for follow path** (2026-10-13,
  docs/specs/image-arcs-centrelines.md, schema 20): images get `trace` (outlines or centre
  lines) and `arcs`; v19 images migrate as outlines without arcs. A greedy line-and-arc
  fitter in the core (`fit::fit_path`); centre lines from the V-carve medial axis with
  pass-wise tidying (`chipload_toolpath::centre`), dots counted, stroke widths reported;
  only a follow path may cut a centre-line layer. CLI `--trace`, `--arcs`; app: Trace and
  Arcs in the image window. Golden v20_image_centre_line. Later: a follow path that goes on
  through a junction instead of retracting.
- **The last pocket refusals fixed** (2026-10-13, docs/specs/offset-retry.md §7): offsets that
  fail at one distance and the next few nanometres get three 10 nm nudges after the 1 nm ones, and
  rest machining's own shrinks are retried farther. The two seeds found by soaking pass, as
  explicit tests; goldens unchanged.
- **Import image from a typed path** (2026-10-13): *File > Import image from path…*, for
  systems without a native file dialog (WSL, minimal Linux).
- **Images traced into shapes for V-carving** (2026-10-13, docs/specs/image-trace.md,
  schema 19): PNG, JPEG or BMP into black and white (threshold auto or set, invert, blur,
  smallest piece, smoothing), traced by marching squares into closed loops that never cross,
  stored with the picture (`drawing.images`) as path shapes on its layer. `chipload image
  add/set/list/preview/remove`; `set` traces again. App: File > Import image… (live preview),
  Job settings > the image's row (Edit…, Remove), View > Show images. Golden
  v19_image_vcarve. Later (§10): arc fitting, centreline tracing, several thresholds into
  layers.
- **Rest pocket refusals fixed** (2026-10-13, docs/specs/offset-retry.md §6): pocket
  shape offsets get a last retry with a tight slice tolerance, and a stray island's offset
  that touches no area is dropped. Three rest pockets the property test found refused now
  clear; two plain pockets that the tight tolerance as a default would break guard against
  that. Golden files unchanged.
- **Material left → the simulation** (2026-10-13, docs/specs/material-left-merge.md): the
  geometric material-left display, its computation, the View toggle, the "can't reach"
  report lines and `plan --json`'s `material_left` are gone; the simulation is on at start
  and shows what the tools leave. The missed-material warning stays.
- **DXF fuzzing** (2026-10-13, fuzz/README.md): a cargo-fuzz target on the importer. Found
  a 127-digit coordinate overflowing the contour joining: entities beyond 1 km (or not a
  number) are now skipped and reported, and the grid lookup saturates.
- **User guide** (2026-10-13, docs/specs/user-guide.md): mdBook in `docs/book/`, every page
  written; generated reference pages (CLI, project, tools, machines, posts) and a coverage
  test; build `packaging/docs/build-book.sh` (zip for releases), screenshots
  `packaging/docs/screenshots.sh`, publishing `publish-book.sh` once the repository is
  public. Help > User guide.
  The file formats' doc comments were rewritten for users on 2026-10-13 (they are the
  reference pages' and the JSON Schemas' descriptions).
- **Add rest pocket** (2026-10-13, docs/specs/rest-machining.md §10): from a pocket, a rest
  pocket right after it with the same shapes, depths and settings, a smaller tool, rest
  after the pocket's tool; library cutting values when the tool has them. GUI: the
  operation card's right-click menu; CLI: `op add-rest`. Copies at creation (option B);
  a live link to the operation (A) only if drift turns out to be a nuisance.
- **Scripting** (2026-10-13, docs/specs/scripting.md; user guide docs/book/src/scripting.md):
  - `--json` on every command, with one envelope (`chipload_json`, `ok`, `notes`,
    `written`, and `error` with the problems' paths) and each command's own keys;
  - `get` and `set` by path (0-based, as problem paths), checked before writing;
  - `--set` for one run of `run`, `plan`, `simulate` and `sheet`;
  - `check-project`;
  - JSON Schemas for projects, tools and machines (`chipload schema`, `docs/schemas/`),
    generated with schemars and validated against every frozen sample in tests (boon,
    a dev-dependency only);
  - no batch command: recipes for bash and PowerShell instead.
- **Rest machining** (2026-10-13, docs/specs/rest-machining.md): a pocket can rest after a
  larger project tool (`rest: {after_tool}`, an optional field, no schema bump). It then
  clears only what that tool's size leaves (rounded corners, slots and gaps it doesn't fit
  in), each leftover grown by the small tool's diameter and pocketed with rings. Pieces too
  small for the small tool too are skipped with a note. Problems: a missing, non-endmill or
  not-larger rest tool, rest with dogbones. Warning: no earlier pocket cuts the same shapes
  with that tool as deep. `chipload op rest`; GUI "Rest after" in the pocket's Clearing
  section; setup sheet. New goldens `v18_rest` (G-code, lines, sheet); existing goldens
  unchanged.
- **Playback slider** (2026-10-13, docs/specs/playback.md): a slider by time under the
  canvas over the last program: moves done drawn as today, the rest faint, the tool (its
  size; a cone for V-bits and drills in 3D) at the current point, marks at operations and
  tool changes, a readout (time, operation and copy, X Y Z, feed); ← → by move, Shift by
  mark, Home/End; play at ×1/×10/×100. With the simulation on, the stock as cut so far
  from per-cell histories recorded with the simulation (both directions cost the same,
  about 6 ms a frame on a 600 × 400 sheet). `chipload simulate --after N`.
- **Material-removal simulation** (2026-10-13, docs/specs/simulation.md): the stock as a
  height map cut by every planned move with its tool's shape (flat endmill, V-bit cone with
  its tip, drill with a 118° point), the lowest point per cell worked out exactly. View →
  Show simulation (and the toolbar's "Simulation") shows it as a shaded image in top view
  and a shaded surface in 3D, computed in the background after each generation; the report
  gets the volume removed and the deepest cut. `chipload simulate` prints the numbers and
  writes a PNG (`png` crate, already in the tree). Display only.
- **Dogbone and T-bone corners** (2026-10-13, docs/specs/dogbones.md): profiles and pockets
  take `corners` (dogbone or T-bone, up to `max_angle`, default 100°). In every inside
  corner on the tool's side the ring goes out to a tip from which the tool's edge touches
  the corner (moved by the clearance) and back: along the bisector, or along the longer
  wall. Profiles: every ring, the roughing rings `stock` short. Pockets: the wall rings,
  islands included. Tabs slide off the legs. A leg that would cut into another contour is
  refused; a corner the tool can't reach gets a note. Golden v18_dogbones.
- **Holes: drilling and helical boring** (2026-10-13, docs/specs/holes.md, schema 18): a
  Holes operation cuts the selected circles; a drill drills those of its size (±0.1 mm) at
  the centre, in pecks that rise to the safe height and rapid back into the hole to 0.5 mm
  above the depth drilled; an endmill bores them with a helix at the wall (at most the step
  per turn, never steeper than the ramp angle) and a level circle. Holes more than twice the
  tool leave a core: noted in a through hole, refused in a blind one. Plain G0/G1, so GRBL
  runs them too. The checker allows a rapid only back into the hole the tool just came out
  of. GUI: "+ Holes", the method follows the tool. Golden v18_holes.
- **Finishing pass and onion skin** (2026-10-13, docs/specs/finish-and-skin.md): profiles
  can rough with stock left on the wall and finish it in one level lap, and stop a skin
  above the end depth that is cut last; roughing, then finishing, then skins, all leaving
  the tabs. GUI section "Finish and skin", setup sheet text, golden `v17_finish_skin`.
- **Follow path** (2026-10-13, docs/specs/follow-path.md, schema 17): open (and closed)
  paths cut on the line, back and forth or descending while travelling.
- **Spiral profiles and ramped tabs** (2026-10-12, docs/specs/profile-spiral.md): ramped
  profiles spiral forward down the ring (no back-and-forth), one lead-in, a rise-out at
  the end; tabs have a flank angle (schema 16) and the tool follows their trapezoid.
  Commits 160b6c1 (schema), aeeb1b9 (toolpath), and the GUI field and setup sheet text.

- **Colour theme and contrast:** Settings → Colour theme (System, Light, Dark; moved from
  View → Theme). Every kind of text now reads at least 4.5:1 in both themes (brighter dark
  text, weak text and errors; darker light-theme warnings and errors; white on the dark
  accent; subtler card hover), and every canvas line at least 3:1 (lighter stock outline,
  less fading for stale toolpaths and copies, 3D depth shaded from light to the plain
  colour instead of to near black). Tests check the ratios (`style.rs`, `view.rs`).
- **Small items of the improvement pass** (docs/specs/small-items.md): Help → Keyboard
  shortcuts (F1, from one table); drop a project or DXF on the window; View → Measure (M);
  copy, paste and mirror drawn shapes (Edit menu, Ctrl+C/V, `chipload shape copy/mirror`);
  the chip load (mm/tooth) beside the feed, on the setup sheet and in `chipload plan`.
- **Setup sheet for the operator** (docs/specs/setup-sheet.md): an HTML page next to the
  G-code (`part-sheet.html`): stock, X0 Y0 in words on the stock, Z0, a top-view picture
  (parts, each file's cuts, tabs, the zero), each file in order with the tool to load and
  its operations, depths, feeds, entries, tabs, times and deepest cut, then the warnings
  and the time. Written by `chipload run` (`--no-sheet`), `chipload sheet` and GUI export
  (Settings → Write a setup sheet with the G-code). Posts can carry a `note` for it.
  Later: stock material and free notes on the sheet (new project fields), PDF, opening the
  sheet from the GUI.
- **Cycle time estimate** (docs/specs/cycle-time.md): total, cutting, rapids, spindle and
  tool changes, per operation and per file, at the programmed feeds (rapids at the max feed,
  else 5000 mm/min assumed); in the report, on the operation cards, in `chipload run/plan`
  and `plan --json`. Later: a rapid-rate setting.
- **Problems shown where they are, while editing** (docs/specs/live-problems.md): after
  each finished edit every rule that needs no geometry is checked; a bad field gets a red
  frame and its message on hover, its section a red dot, the operation card a line, the
  Job row a count, and a Problems strip lists them all (click to go there). The live
  preview waits until they are fixed. Later: warnings (amber) the same way.
- **Your own post-processors, and LinuxCNC and Mach3/4 posts** (docs/specs/user-posts.md,
  ADR 0016): posts from `posts/` in the settings folder, copied into the project when chosen
  (schema 15, Update when the file changes); the checker accepts LinuxCNC/Mach setup codes;
  `chipload post list/use`. The new posts are untested on real controllers (their header
  says so). Later: one file with tool changes (G43) for LinuxCNC/Mach.
- **Recovery copies and a backup on save** (docs/specs/autosave-and-backup.md): while a
  project has unsaved changes, a copy is kept in the settings folder (2 s after each edit);
  after a crash, *Recover unsaved work* offers it at the next start. Saving over a project
  keeps the previous file as `.bak` (GUI and CLI).
- **Generating in the background, and live preview** (docs/specs/background-generation.md):
  the window stays live while planning, ■ Cancel stops a run, a crash while planning
  becomes a report error; the toolpath updates by itself 0.5 s after an edit (View →
  Update toolpath automatically, settings schema 3), the old one faded meanwhile.
  Follow-up: cancel checks inside the new adaptive planning when stage 2 lands.
- **Polyline handles easier to read**: the segment middles are round handles in their own
  colour (the points stay squares), and a selected polyline shows how to drag them (move a
  point, insert one, Shift to bend) under its form and in the status bar.
- **Rounded corners on the clearing rings of a pocket** (docs/specs/pocket-round-corners.md):
  every ring but the wall ring gets its corners rounded (radius min(k·s, D), less where that
  would leave material), for smoother motion; per pocket, on for new ones (schema 14).
- **Clearer left sidebar** (docs/specs/sidebar-sections.md): the Job row on top, then
  Operations (with + Profile / + Pocket / + V-carve for the selection) and, below a
  draggable split, Shapes (selection, shape editor, layers with counts).
- **Material left lost behind a bay of the tool's reach** (bug found 2026-10-12 by the
  material-left property test): the offset library returned a loop that closes while
  growing as an open polyline, which was dropped; it is now closed (ends within 0.001 mm)
  or an error. Display only, toolpaths were not affected.
- **Import/export of the tool library and machine profiles**
  (docs/specs/library-import-export.md): files in the library's own formats; import adds
  new entries and keeps yours unless asked to replace (`chipload tool export/import`,
  `chipload machine export/import`, GUI Import…/Export… in the library windows).
- **A tool per operation** (docs/specs/tool-per-operation.md): each operation cuts with its
  own tool, in tool groups. Output is one checked file per tool, or one file with `T M6`
  for senders that handle the change (machine setting, project schema 12, machines.json 2).
  CLI `op tool`, `run --tool-change`, `check --tool`; GUI *Tools* section and an operation
  *Tool* box.

- **Scaling the imported drawing** (docs/specs/drawing-scale.md): uniformly, about its
  centre, by a factor or to a width or height; updates re-approximate curves at the new size
  (schema 11, `chipload place --scale/--width/--height`, GUI *Size* rows).
- **Trochoidal vs adaptive:** dropped 2026-10-11; adaptive covers it, trochoidal stays as
  it is.
- **GUI pass** (approved 2026-10-11): report under the canvas, forms in sections, operations
  as cards, a toolbar row and start screen, a canvas legend, View > Theme (remembered).
- **V-carving, step 1** (docs/specs/v-carve.md): a V-bit follows the centre line of closed
  shapes, as deep as makes the cut as wide as the shape (schema 10, V-bit angle and tip,
  tools.json 2). The next steps are under Later.
- **Placing the imported drawing** (docs/specs/drawing-placement.md): move it by an offset or
  align it with the stock, kept through updates from the DXF (schema 8, `chipload place`).
- **Turning the imported drawing in 90° steps** (docs/specs/drawing-rotation.md): about its
  centre, exact (schema 9, `chipload place --rotate`, GUI ⟲ ⟳).
- **Start points, lead-in/out, helical entry** (docs/specs/helix-and-leads.md), in 0.9.0.
- **Offset retry** (docs/specs/offset-retry.md): a pocket or profile whose offsets hit a
  numerical coincidence is retried with offsets moved by 1e-6 mm instead of refused.

## Later

- **Ring corner radius per corner:** a ring now has one radius, set by its sharpest corner;
  rounding each corner by its own angle would let acute shapes (triangles) keep larger
  radii at their other corners (follow-up to docs/specs/pocket-round-corners.md).
- **A list of every shape** in the Shapes section of the sidebar, drawn and imported, to
  click and select (docs/specs/sidebar-sections.md decision 5, user 2026-10-12).
- **Several DXFs in one project:** import more than one drawing into a project, each with
  its own source and placement (user request 2026-10-11).
- **Import SVG** as well as DXF, working the same way: units, contours, the import report,
  update from a new version, placement (user request 2026-10-11).
- **Voronoi elsewhere:** investigate whether the segment Voronoi diagram (boostvoronoi, ADR
  0015) can make other parts faster or more robust: pocket clearing and adaptive, material
  left, narrow-part checks, distance queries (user request 2026-10-11).
- **V-carve: text typed in the app** (docs/specs/v-carve.md, step 2): a text shape with a
  font, size and position, its outlines V-carved. Needs a font decision (bundle an OFL font,
  or use the system's fonts) and a schema bump. Not prioritised (2026-10-11).
- **V-carve: tracing a black-and-white picture** (step 3): outlines from a bitmap, with the
  pixel size in mm chosen explicitly. Not prioritised.
- **V-carve: a flat floor** for strokes wider than the bit carves: the V-bit cuts the walls,
  an endmill clears the floor. Needs a tool per operation. Not prioritised.
