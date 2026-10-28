# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

- **Translations** (docs/specs/translations.md, ADR 0022), asked for 2026-10-24: the app in
  German, French, Spanish, Italian and Swedish. **Step 1 built 2026-10-24:** the
  `chipload-i18n` crate (Fluent, texts compiled in), every text of the GUI as `t!(…)`
  (about 1,180 texts in 34 `.ftl` files), the five translations (machine-made from
  `crates/chipload-i18n/GLOSSARY.md`), View > Language and *This computer > Preferences*
  (the system's language by default, `language` in `settings.json`), a decimal comma in
  fields in those five languages, the guide page *Languages*. Left:
  - The user reviews Swedish; the other four wait for native speakers who machine
    (marked *review wanted* in the menu; `Language::reviewed`). The translators' open
    suggestions are at the end of the glossary.
  - **Step 2:** messages from the library crates (problems, report lines, notes, checker,
    import report, `operation_label`, error texts, `Link::text`), and the GUI's own
    problems in `EditJob::missing()`, which are worded like them. Typed length arguments
    replace `text_in`. Planned in the spec, §7 (decisions 11–17, order of six commits),
    approved 2026-10-24. Done: 7.3.1 (the `Message` type, the frozen English snapshot)
    and 7.3.2 (problems), 7.3.3 (planning notes, the time estimate and chip loads, the
    drawing's facts and the DXF import's issues), 7.3.4 (the checker's findings), 7.3.5
    (errors, library and recovery messages, the reports of changes to the drawing), 7.3.6
    (`text_in` deleted; the simulation's lines, the work zero line, hovers with a length
    and the tool types and copy order in lists follow the language and display units).
    **Done.**
  - **Step 3 done 2026-10-24:** the setup sheet in the app's language (`sheet.ftl`;
    `chipload sheet --lang`, `run --sheet-lang`; a German golden sheet).
  - **Everything translated** 2026-10-24, the five languages complete; the ribbon fits
    in every language at the default window size (German ribbon buttons *Import* and
    *Export*, the French VIEW caption *Vue*).
  - **Left:** native review of all five (the user reviews Swedish; the others are marked
    *review wanted*; open suggestions at the end of the glossary). Still English on
    purpose: the issues a DXF had when it was read (stored in the project as words;
    a schema change would be needed: the user chose 2026-10-25 to leave it unless users ask), migration reasons and a library file's rule
    violations, what other libraries write (file system, JSON, TOML, pictures), a new
    text's starting word "Text", startup errors before the window opens. A rare English
    slip kept word for word: "the centre line centre line" in
    `toolpath-error-medial-name` for a traced centre line.

## State to remember (2026-10-19)

- **Releases**: the latest published is 0.27.1 (2026-10-27); before it 0.25.1 (2026-10-25) and 0.24.0 (2026-10-24, the translations),
  built by Actions: Linux and Windows x86_64 and ARM64, macOS universal, the guide zip,
  `SHA256SUMS.txt`; the guide is online from `v0.24.0`. The first v0.24.0 tag failed on the
  message snapshot (platform digits); the tag was moved to the fix, 5b647c2, before any
  release. 0.22.0 was tagged but withdrawn unpublished. **0.25.0** (explode a DXF, join
  ends; the drawn shapes' box, Position and Turn, Browser rows and names; French
  corrections) published 2026-10-25. **0.25.1** (French *Contour* everywhere, closed
  shapes as drawn and chaining per layer, explode keeping half circles) published
  2026-10-25.
  **0.26.0** (dogbones chosen per corner; start points and corner choices follow moved
  drawn shapes) tagged 2026-10-26, draft release from CI.
  **0.26.1** (bones beside a contour that runs through a pocket or lies on its wall)
  tagged 2026-10-26, draft release from CI.
  **0.27.0** (tabs placed by hand; dogbones follow a dragged shape; bones beside
  contours the operation cuts through) tagged 2026-10-26, draft release from CI.
  **0.27.1** (a bone near another contour is a warning, only cut contours count) published
  2026-10-27, its notes covering 0.26.0 to 0.27.1; the 0.26.0, 0.26.1 and 0.27.0 drafts
  stay unpublished.
  Old tags remain (`v0.3.0` on, and the guide-only tags `guide-0.21.0-1`, `-2`,
  `guide-0.22.0-1`).
- **The repository is public, under the `chipload` organisation** (2026-10-17):
  github.com/chipload/chipload, the guide at chipload.github.io/chipload (published from
  v0.21.1; guide-only fixes go out with a `guide-X.Y.Z-N` tag and
  `packaging/docs/publish-book.sh`). Actions run (free for public repositories): a `vX.Y.Z`
  tag builds every file and drafts the release; *Actions > Release > Run workflow* builds
  without a release. The `gh` token's resource owner is `chipload` (Contents, Actions and
  Pages read/write).
- **For the user to do on GitHub**: the organisation's avatar
  (`packaging/brand/png/chipload-avatar-512.png`), its name, description and website, and
  the repository's social preview (`packaging/brand/png/github-social-preview.png`).
- **Windows builds** (local fallback, without Actions): Rust (msvc) and the Build Tools are
  installed on the Windows side, checkout `C:\code\chipload`; the steps are in
  docs/releasing.md (*Without GitHub Actions*).
- **Branches**: `release-0.12` (the 0.12.1 fix branch, kept for history). Local tags
  `adaptive-final` (main before trochoidal and adaptive were removed) and `python-final`.
- **Waiting on testers**: real-controller tests of every post (GRBL, LinuxCNC, Mach3/4), inch
  G-code, and the new builds (macOS, ARM64 Linux and Windows); the README asks for them.
- **Scope** (ADR 0017): a lean power tool.
- **Disk space** (2026-10-19): `target/` had grown to 322 GB and WSL's `ext4.vhdx` on C: to
  360 GB (C: 94 % full). The user ran `cargo clean`; the image shrinks only after
  `wsl --manage Ubuntu --set-sparse true` (or a `diskpart` compact) on the Windows side.
  Run `cargo clean` now and then, and after a Rust update.

## Next

- **Surface property test failed on CI** (Windows, reported by the user 2026-10-27):
  `passes_cover_the_stock_and_go_down_beside_it` (crates/chipload-toolpath/tests/surface.rs)
  says "down at 0.25, move 2" for x0 = 0, y0 = 0, w = 291.88231113195496,
  h = 291.8824350298392, radius 0.5, fraction 0.1, depths 1, start (0, 0); proptest seed
  `cc ba0687aa62cbaf13b2dc560882d4c7eb1cb475775a1334494ebaa69e6f4f5c20`. The stock is square
  within 0.00012 mm: likely the test and `surface_moves` disagree on which way the passes
  run when w and h are that close (the test takes `w >= h`), so the test checks the wrong
  axis. Not checked yet: reproduce with the seed, see which way the code runs, and fix
  the test or the code (a plunge on the stock would be a real bug), with the seed kept in
  the regressions file.
- **The legend runs under the navigation bar in a narrow canvas** (seen 2026-10-28 in the
  kittest pictures, `pictures` in crates/chipload-gui/src/app/tests/layout.rs): in German at
  the default 1200 × 800 the canvas is about 420 px wide, the open legend about 190, and the
  bar can't keep to its right, so it covers the legend's fifth row. Older than the drawing
  work. Ideas: the bar above the legend when there's no room beside it, or the legend
  folded by default in a narrow canvas; a layout test like the ribbon's.
- **Traced loops cross on a tiny picture with arcs** (found 2026-10-28 by a local
  workspace run): `traced_loops_separate_black_from_white`
  (crates/chipload-image/tests/trace.rs) fails `no_crossings` for w = 4, h = 5, smoothing
  0.135, arcs on, the black pixels (row by row from the top) at indices 15, 26, 27, 37, 38,
  39 and 48 of `bits`; proptest seed
  `cc 2c938f78133a9767c000bb3dba47553796243e75bc66d9f341e2809614ebb474`. A diagonal
  staircase of a few pixels: likely two arcs fitted to neighbouring loops bulge into each
  other where they pass a shared corner. Not checked yet: reproduce, see whether the
  crossing is in the fitted arcs or already in the loops, fix, and add the seed to
  `trace.proptest-regressions` with the fix (left out until then so main stays green).
- **A move more on macOS** (found 2026-10-24 by the release build): `chipload plan
  tests/corpus/jobs/v25/rounded_rectangle.chipload` plans 5041 moves on Linux and Windows,
  5040 on macOS, with byte-identical G-code (the golden passes everywhere): likely a
  zero-length move, from a last-digit difference in the platform's maths library. Find
  which move and whether the planner should drop it everywhere.

- **Drawing in Chipload, self-sufficient** (approved 2026-10-27,
  docs/specs/drawing-roadmap.md; ADR 0017 amended, ADR 0023): phase 1, precise input and
  the basic tools (docs/specs/drawing-input.md), **done and pushed 2026-10-27**: the
  `chipload-draw` crate with more snaps; keys, typed values and angle steps; box
  selection and moving with the mouse; arc and slot tools; polygons (schema 30); lengths
  on the canvas. Not tried by hand in the running app beyond screenshots; the guide's
  screenshots show the old ribbon until `packaging/docs/screenshots.sh` runs again.
  Phase 2, editing (docs/specs/drawing-editing.md), **done 2026-10-28, not pushed**:
  offset, fillet and chamfer, trim, extend and break, union, subtract and intersect,
  grid and polar arrays, align and distribute, each on the command line and in the
  window; golden v30_drawn_edited drawn with these commands alone. Next: phase 3
  (SVG import through `usvg`, added 2026-10-27; spec approved 2026-10-28,
  docs/specs/svg-import.md, ADR 0024: **done 2026-10-28, not pushed**: the reader crate,
  schema 31, the command line and the window; follow-ups: texts that still say DXF for
  any drawing file (placement "as in the DXF", Browser and form hints, "DXF 2" in the
  selection summary), run the new svg_import fuzz target, try it by hand with real
  Inkscape and Illustrator files), 4 (designs, a box maker with finger
  joints first, extensible to more designs, docs/specs/designs.md, and parameters,
  docs/specs/parameters.md: both drafted 2026-10-28 and **on hold** by the user the same
  day), 5 (DXF export and construction geometry: spec approved 2026-10-28,
  docs/specs/dxf-export.md; DXF export and construction geometry (schema 32) done
  2026-10-28, not pushed; follow-ups: try the exported DXF in other programs (LibreCAD,
  Inkscape, LightBurn, Fusion), keep the construction flag through a clipboard paste).
  **Next (the user, 2026-10-28): step back and think about the app's direction, the
  workflow now and what we want in the future, before more features.**
  CI runs only when started by hand since 2026-10-27 (Actions minutes).

In order (proposed 2026-10-17; the user may reorder); each with a spec first.

1. **Feeds and speeds per material** in the library, with a chip-load warning (lean plan
   4, ADR 0017); with it a chip load range per tool (amber warning, schema change).
2. **Operation presets and layer rules**: paused 2026-10-13 by the user, spec drafted
   (docs/specs/presets.md, decisions not reviewed).
3. **Chipload in the browser** (docs/specs/web.md): step 1 done 2026-10-13 (the libraries
   compile for wasm32, checked in CI). Step 2, the browser app (a Web Worker, upload and
   download, browser storage, a bundled font): proposed again 2026-10-17 as a way to test
   without installing, the user said wait. Its decisions are proposals to review then.
4. **Image fuzzing got slow** (2026-10-13): 38 inputs a second with arcs and centre lines,
   against thousands before; find which pictures are slow (centre lines on noise?) and
   whether a user could hit it.

## Done

- **A bone near another contour: a warning, and only cut contours count** (2026-10-26, the
  user): a dogbone or T-bone that also cuts into another contour, or another wall of its
  own, is cut with a warning instead of stopping generation; contours no enabled operation
  cuts don't count at all (dogbones.md decision 6 as changed).
- **Dogbones follow a dragged shape; bones beside contours the operation cuts through**
  (2026-10-26, the user): corner choices, start points and tabs placed by hand now follow a
  drawn shape changed by its own handles or form (a corner keeps its choice when a rectangle
  is moved, turned or resized); a contour that meets the operation's cut within the bone's
  reach no longer stops the bone (docs/specs/dogbones.md §6).
- **Tabs placed by hand** (2026-10-26, docs/specs/tabs-by-hand.md), the user's question
  whether tabs drag like a ring's start: `tabs.at` (optional, no bump); a contour with
  points gets exactly those tabs, no sliding, refused when too close, on the start or off
  every contour; `chipload op tabs` and `op tab`; on the canvas, tabs drag along their ring,
  right-click removes, spaces evenly or adds; they follow `place` and moved drawn shapes.
- **Bones beside a contour that runs through the pocket** (2026-10-26, the user, in 0.26.0):
  a pocket that the part's outline crossed near a corner got no dogbone or T-bone there
  ("would cut into contour #2"), though the pocket clears that line anyway. In a pocket,
  another contour now counts only outside the pocket's area, and for any operation not
  where it lies on the operation's own line (a shared edge) (docs/specs/dogbones.md §6).
  On `shared_edges.dxf` every rectangle sharing a wall now generates.
- **Dogbones chosen per corner** (2026-10-26, docs/specs/dogbone-per-corner.md), after a
  user's slot with a short step at each end couldn't get dogbones or T-bones with a 6 mm
  tool: any corner of a profile or pocket can have its own bone (round, dogbone, T-bone
  along either wall), also past the max angle, so an operation can also start round and get
  bones only where picked. `corner_choices` (optional, no bump); `chipload op corners` and
  `op corner`; diamonds on the canvas with a menu; corners the tool can't reach shown in
  amber; the error says when a bone reaches its own contour. Start points and corner
  choices now follow drawn shapes that are moved, turned or scaled.
- **Closed shapes as drawn, chaining per layer** (2026-10-25,
  docs/specs/chaining-per-layer.md), a user's DXF with pockets drawn against the outline
  on another layer: closed polylines and circles are contours as drawn, loose lines chain
  and are de-duplicated only within their layer. The user's file is in the corpus
  (`tests/corpus/dxf/shared_edges.dxf`, with their consent).
- **Explode kept a half circle a hair over π** (2026-10-25, found with the same file): its
  bulge came out just over 1 and the shape was dropped as "too short"; clamped to ±1.
- **French: *Contour* for the profile operation everywhere** (2026-10-25, the user):
  *contournage* became *contour* in every French text (21 lines) and the glossary.
- **Move, turn and size drawn shapes; drawn shapes in the Browser, with names**
  (2026-10-25, docs/specs/drawn-shapes-box.md), the user's request: a box with move, turn
  and size handles round a selected polyline or several drawn shapes (`shape move/turn/
  scale`); Position and Turn in Properties; the Browser lists drawn shapes per layer, and a
  drawn shape can have a name (optional field, no schema bump; `shape set --name`). Texts
  in six languages (machine-made; the user reviews Swedish), guide.
- **Explode a DXF, join close ends** (2026-10-25, docs/specs/explode-and-join.md), a
  user's request: `chipload dxf explode` and *Explode into shapes…* turn a DXF's shapes
  into drawn circles and polylines (the project forgets the DXF; operations keep their
  shapes, with the same G-code on the corpus); `chipload shape join` and *Draw > Join
  ends…* join open polylines whose ends are within a gap (default 0.01 mm, at most 1 mm),
  same layer, clear pairs only. A closed polyline may now have two points when a segment
  bends (a DXF circle of two half circles). Texts in all six languages (the user reviews
  Swedish), guide page *Exploding a DXF, joining ends*.
- **Release file names** (2026-10-24, ADR 0010): the app is `chipload-X.Y.Z-…` (AppImage,
  `.exe`, `.dmg`) and sorts first on the release page; the command line is
  `chipload-cli-X.Y.Z-…`, always an archive (Windows now a `.zip` with LICENSE and the
  notices, not a bare `.exe` that flashes a console when double-clicked). The release notes
  start with "Download the app"; the install page, README and docs/releasing.md follow.
  Not done: a message when the CLI starts with no arguments on Windows (optional; it would
  wait for Enter in a terminal too).
- **Smaller debug builds** (2026-10-19): `[profile.dev]` keeps line tables for
  our crates and no debug info for dependencies; after a full test build `target/` is
  3.1 GB. A debugger no longer shows local variables.
- **A narrow Properties panel and folding side panels** (2026-10-17, 0.22.2): every
  Properties page fits the least width (250 px; rows wrap, long names end in …); P or
  *View* folds both side panels for a larger canvas (docs/specs/browser-and-layout.md).
- **A text's operations name it, and the canvas says when the simulation updates**
  (2026-10-17, 0.22.2; user report on a V-carve that seemed not to update after a stock
  change): the selection read "no geometry" for a text, and a slow simulation had no sign.
- **An architecture overview for contributors** (2026-10-17): `docs/architecture.md`, the
  layers as a diagram, the steps of `run_job` and the crates; linked from the README and
  CONTRIBUTING.md, kept right by a CLAUDE.md rule.
- **Add rest pass from the Browser** (2026-10-17, 0.22.1; hidden in 0.22.0, which was
  withdrawn): right-click an operation for project tools and tools from the library, and
  the reason when it can't (docs/specs/rest-passes.md §6).
- **Two toolpath fixes found with the fonts** (2026-10-17): an empty or broken pocket offset
  (the offset library on a sharp notch where an arc meets a line, Smokum's "n") is retried
  with the arcs as chords (docs/specs/offset-retry.md §8); a V-carve centre line is split
  until its quarter points are within tolerance, so the groove no longer comes out up to
  0.04 mm too wide where the depth meets its cap (golden v19_image_vcarve gained points).
- **More fonts** (2026-10-17, docs/specs/more-fonts.md, ADR 0021): ten Apache-2.0 fonts
  (Roboto Slab and Bold; scripts Satisfy, Yellowtail, Rochester; display Luckiest Guy,
  Ultra, Chewy, Smokum, Cherry Cream Soda, Permanent Marker), the font list grouped and
  drawn in each face; overlapping letter outlines merged (`merge::merge_overlaps`); golden
  v29_script_text. Released in 0.22.1.
- **Surface** (2026-10-17, docs/specs/surfacing.md, schema 29): flatten the spoilboard or
  face the stock: the whole stock, back and forth along its longer side with a wide
  endmill, going down beside it; cut once whatever the copies; first in its part. Ribbon
  *Surface* (no selection), `op add-surface`; golden v29_surface. Released in 0.22.1.
- **Commercial-CAM features** (user, 2026-10-13: "1-5 and 7 we should add"): follow path,
  finishing pass and onion skin, drilling and helical boring, dogbone and T-bone corners,
  the material-removal simulation and playback (entries below); tested by the user, all
  okay (2026-10-17). Surfacing came later (see Surface).
- **Lean plan** (approved 2026-10-13, ADR 0017): rest machining, scripting, material left
  merged into the simulation and DXF fuzzing done (entries below); feeds and speeds and
  presets are under Next.
- **General improvement pass** (approved 2026-10-12): background generation and live
  preview, recovery copies and `.bak`, user posts, problems while editing, cycle time,
  the setup sheet, the small items, playback and inches (entries below).
- **Trochoidal and adaptive clearing removed** (2026-10-13, ADR 0017): the code is at the
  local tag `adaptive-final`; since schema 26 such a pocket loads as a rings pocket.
- **Property tests with a fixed seed for releases** (2026-10-13): release builds run them
  with a fixed seed (`packaging/test-release.sh`), so a release can't fail on an unseen
  case; everyday runs keep drawing new cases, and failures are saved next to each test
  file to commit with the fix.
- **Rest passes under their pocket** (2026-10-17, docs/specs/rest-passes.md): a pocket's
  rest passes follow it, shown as leaves under it (*1.1*), cut after the run of same-tool
  pockets; `op add-rest`, `op rest-pass`, `op make-rest-pass`; golden v28_rest_passes.
  Replaces *Add rest pocket*'s copy; a standalone rest pocket stays for a pocket cut
  elsewhere.
- **Brand and the public repository** (2026-10-17): a logo (icon and wordmark, inspired by
  the GNTECH logo; `packaging/brand/`) in the app icon, window, README and guide; the
  repository moved to the `chipload` organisation and was made public; the guide published
  at chipload.github.io/chipload; the README asks for tests on real machines and says that
  what you make with Chipload is yours; the organisation's profile page, topics and social
  preview; a showcase picture with the logo V-carved and the toolpaths shown.
- **Release builds for every common platform** (2026-10-17, ADR 0010): Actions builds
  Linux and Windows on x86_64 and ARM64, macOS universal (`.dmg` and CLI), and the guide
  zip; first built for 0.21.1. Release files named in lowercase from the next release.
- **Inches, messages** (2026-10-17, docs/specs/inch-units.md step 4): report notes, checker
  warnings, problems and the setup sheet's *Watch out* list follow the display units; every
  length in a library message is written with its unit and converted where it is shown.
- **Inches** (2026-10-17, docs/specs/inch-units.md, schema 28, machines.json 4): G-code in
  G20 per machine (post `units`/`inch_decimals`, the checker reads G20); display units in
  the app (Preferences > Units: fields with fractions, grid, canvas, setup sheet, inch
  defaults for new projects); lengths with a unit on the command line (`sheet --units`,
  `run --sheet-units`). In 0.21.0.

- **Profiles always spiral** (2026-10-15, schema 27, docs/specs/profile-always-spiral.md):
  the straight plunge and the lead are gone from profiles (no worse options); a profile
  has a `ramp_angle` instead of an entry. A plunged project spirals at 3° after migration.
  Where a stretch of the spiral or a tab splits a chord of an arc, the point is put onto
  the arc, so circles still come out as (helical) arcs. Every plunged profile's golden
  changed; `v26_straight_tabs` frozen.

- **The profile's spiral ramps at the angle and levels only the ramp** (2026-10-15,
  docs/specs/profile-ramp-level.md): each pass ramps over the length its step needs and
  runs level for the rest of the lap, and the whole level lap at the end is one stretch
  the length of the ramp; the skin lap the same. One lap less per ring (a quarter to half
  of the cut). Every spiral profile and skin golden changed.

- **The pocket's rings as one spiral** (2026-10-15, docs/specs/pocket-spiral.md): a ring
  steps out onto the ring outside it over its last two diameters instead of closing, where
  that is checked clear of the walls and leaves nothing uncut; the link move goes with it.
  The drifted stretch is re-fitted with arcs. Island rings and the wall ring still close.
  Every pocket golden changed; thirty retracts fewer in the corpus.

- **No worse options: the pocket's clearing settings are gone** (2026-10-15, schema 26,
  docs/specs/pocket.md §9): the tool stays down wherever the verified link is safe, the
  inner rings are always rounded (radius capped at two stepovers), rings is the one way
  (a trochoidal or adaptive project is a rings pocket). Goldens of every pocket with a
  setting off changed; the four frozen removed-strategy projects are goldens again.

- **Review fixes** (2026-10-15, from the user-perspective review of 2026-10-14): one name
  for the stock simulation and automatic updates; one centre-cutting reminder per tool;
  a Through button for the end depth and the V-carve's depth called End depth; Export
  says it writes every part; `shape remove`, `place --image`, the --op numbering said
  plainly; default feeds from the tool diameter; the profile's entry as a combo; the
  clearance field shown as Oversize; copies and parts overlap by their tools' paths;
  a rapid to 1 mm above the stock before every plunge (docs/specs/approach.md, every
  G-code golden changed by those `G0 Z1.000` lines).

- **Refactoring pass** (2026-10-14, 13 commits, no change to any G-code or golden): typed
  problems (`Problem::at`, no re-parsing of "path: message"), notes and toolpath errors
  that carry their operation (`Note`, `ToolpathError::Operation`, `Overlap`), one owner
  for the rest and tab-height rules (`rest_refusal`), `ToolType::name`, `Stock::top_z`,
  `Job::tool`, `Operation::label`, `chipload_core::text`, `Ordered` sort keys; the pocket
  planner on `PocketRing`/`PocketRequest`/`PocketCut`; every cavalier_contours call in
  `cavalier.rs`; `plan_job` in steps; the GUI's state in `Sim`, `Drag` and `Dialogs` and
  its tests split by area.

- **Rounded rectangle corners** (2026-10-14, docs/specs/rectangle-corner-radius.md, schema
  25): a drawn rectangle's corner radius, true arcs, at most half the shorter side; the
  form's Corner radius row and `shape add/set --corner-radius`.
- **A clearer image window** (2026-10-14): the traced parts show near black on white, and
  outlines are no longer drawn over them, so the detail stays visible while setting the
  threshold; centre lines are still drawn, as they are the result. The preview zooms
  (scroll, up to 32×), pans (drag) and fits again (double-click).
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
- **Picture and text handles** (2026-10-17, docs/specs/box-handles.md): a selected
  picture or text shows move, turn (15° steps) and corner size handles; the box follows the
  drag and the change is applied on release, as one undo step. Selection and Properties
  (same day): no Selection section while an operation is open; *Use selection (N)* counts.
- **Text** (2026-10-16, docs/specs/text.md, ADR 0020): a text object like a picture, in
  six built-in GNU FreeFont faces; letters as shapes stored in the project; Draw > Text,
  Properties, Browser rows, `chipload text …`, `place --text`. Later (spec §7): installed
  fonts, single-line engraving, text on a curve.
- **Pocket rings one island at a time** (2026-10-16, docs/specs/pocket-island-order.md):
  a ring waits only for the rings next to it one level in; each island's rings are cut in
  one run, spiralling out to the island; the wall is still last. round_corners: 31 → 23
  retracts, 25 s shorter.
- **MCP server** (2026-10-16, docs/specs/mcp.md, ADR 0019): `chipload mcp` serves every
  command as a tool to AI assistants over stdio, generated from the CLI. Step 2 done the same
  day: the app reloads a project changed on disk (docs/specs/reload-changed-file.md), quietly
  as one undo step, or asks when there are unsaved changes. Not tried with a real MCP client
  yet.
- **Drawing on the canvas bar; Images fixed** (2026-10-16, docs/specs/view-and-settings.md
  §2): Drawing hides the contours to see the stock alone; pictures are painted over the
  simulated stock and in 3D (they were hidden under the stock); the bar shows icons only
  when the canvas is too narrow, clear of the legend.
- **macOS release build** (2026-10-16, ADR 0010): the release workflow builds a universal
  `Chipload.app` in a `.dmg` and a CLI archive (`packaging/macos/build-macos.sh`), signed ad
  hoc. Built since 0.21.1 (after a fix for `otool -L` on universal binaries); nobody has
  started the app on a Mac yet. Later: notarisation, opening files from Finder.
- **A new project drops the old simulation** (0.18.0): New, Open and Import DXF clear the
  stock simulation and playback with the toolpath.
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
  operation card's right-click menu; CLI: `op add-rest`. Copied at creation (option B);
  replaced 2026-10-17 by the live link (A), rest passes (see above).
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
- **Trochoidal vs adaptive:** dropped 2026-10-11 (both were removed 2026-10-13).
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

Follow-ups, not planned yet:

- **Profiles:** a separate finishing feed; rest passes for profiles, if missed.
- **Pockets:** a finishing pass (walls and floor); dogbones with rest passes.
- **Holes** (docs/specs/holes.md §9): canned cycles G81/G83 as a post option, DXF points as
  hole centres, a clearance for a fit, dwell, chip-breaking pecks, V-bit chamfers,
  counterbores in one operation.
- **Simulation:** comparing with the design (gouges), holders and flute length; a G-code
  panel with the current line during playback.
- **Copy imported shapes** (as drawn polylines); today only drawn shapes copy.
- **A bug-report bundle.**
- **A list of every shape** in the Browser's Drawing folder, drawn and imported, to click
  and select (docs/specs/sidebar-sections.md decision 5, user 2026-10-12; the sidebar is
  now the Browser).
- **Import SVG** as well as DXF, working the same way: units, contours, the import report,
  update from a new version, placement (user request 2026-10-11).
- **Voronoi elsewhere:** investigate whether the segment Voronoi diagram (boostvoronoi, ADR
  0015) can make other parts faster or more robust: pocket clearing, narrow-part checks,
  distance queries (user request 2026-10-11).
- **V-carve: a flat floor** for strokes wider than the bit carves: the V-bit cuts the walls,
  an endmill clears the floor (a tool per operation exists now). Not prioritised. (Steps 2
  and 3 of docs/specs/v-carve.md, text and traced pictures, are done: see Text and Images.)
