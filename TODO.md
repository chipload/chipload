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
  being added as collaborators. Decide: make it public, add testers, or another way to
  collect reports (the 0.13.0 notes ask for issues).
- **Branches**: `adaptive-stage2-wip` (local only, rebased on main 2026-10-12, see the
  experimental item below); `adaptive-stage2-wip-backup` (the branch before that rebase,
  can be deleted); `release-0.12` (the 0.12.1 fix branch, kept for history).
- **Waiting on the user**: real-controller tests of the LinuxCNC and Mach3/4 posts.
- **Known test flakiness**: proptests use new random seeds on every run, so a release
  build can fail on a case never seen before (it happened for 0.11.0, 0.12.0 and while
  preparing 0.13.0). Found and not fixed: `material_left_is_out_of_reach_and_complete`
  failed once on a ring pocket (display only, the seed was not kept).

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
     playback (a slider up to a move), comparing with the design (gouges), holders and
     flute length.

  The user will test features 1–4 and 6 and give feedback (2026-10-13).

- **Trochoidal and adaptive clearing: experimental, to be revised** (2026-10-12, user's
  decision). No longer offered as pocket strategies (the GUI offers rings only); projects
  that use them still load and plan as before, with a note that the strategy is
  experimental (no format change). The code stays: trochoidal (docs/specs/trochoidal.md),
  adaptive fronts (adaptive-fronts.md) and the spiral, stage 1 on main
  (adaptive-spiral.md). Stage 2 is on the local branch `adaptive-stage2-wip` (rebased on
  main 2026-10-12; three adaptive tests still fail there; the Chaikin rounding experiment
  in its as-built draft: not adopted). To revise at a later date, with a new spec review:
  the loops' air time (length 2–2.9 × the ideal), then offering the strategies again.
  Rest machining with a smaller tool after that, its own spec. Also found 2026-10-12:
  adaptive refuses an outline with hair-thin spikes (an offset comes back open) that
  rings take; safe (no G-code), seed in `pocket.proptest-regressions`.

- **General improvement pass** (proposed and approved 2026-10-12, in this order):
  1–2. Done (background generation and live preview, see Done).
  3–4. Done (recovery copies and the `.bak` on save, see Done).
  5. Done (user post-processors, LinuxCNC and Mach3/4 posts, see Done).
  6. Done (problems while editing, see Done).
  7–8. Done (cycle time estimate and the setup sheet, see Done).
  - Small items done (see Done). Later from them: copy imported shapes (as drawn
    polylines); a chip load range per tool in the library (amber warning, schema change).
  - Later from the same pass: a bug-report bundle, inch display and entry, operation
    presets, toolpath playback.
- A light canvas for the light theme (the canvas is dark in both themes; its colours are
  tuned for dark). Needs a second canvas palette.

## Done

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
