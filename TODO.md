# TODO

What's planned. Each item gets a spec in `docs/specs/` that is reviewed before it's built
(see `CLAUDE.md`). Reviewed and put in order 2026-11-02 and twice on 2026-11-03 (the user:
"go through todo again and make sure it is tidy and up to date"); *Next* puts usability and
a quick workflow first.

## In progress

- **Translations** (docs/specs/translations.md, ADR 0022): the app, its messages and the
  setup sheet in English, German, French, Spanish, Italian and Swedish since 0.24.0
  (machine-made from `crates/chipload-i18n/GLOSSARY.md`), and Russian since PR #11. Left:
  - **Native review:** the user reviews Swedish as they use it (2026-11-03: a profile's side
    *insida* / *utsida*, oversize *spel*; in the glossary); German, French, Spanish, Italian
    and Russian are marked *review wanted* (`Language::reviewed`). The translators' open
    suggestions are at the end of the glossary. The texts added since 0.31 (statistics,
    cutting data, tool updates, Delete, and 0.33's styles, Stock, tabs along X or Y, folded
    sections, the status hint) are machine-made in every language, Swedish too.
  - **`Language::reviewed`** marks only English, so the menu shows Swedish as *review
    wanted* while the guide (languages.md) leaves it out. Mark Swedish reviewed once the
    user has read it.
  - **Still English on purpose:** the issues a DXF had when it was read (stored in the
    project as words; a schema change would be needed: the user chose 2026-10-25 to leave
    it unless users ask), migration reasons and a library file's rule violations, what
    other libraries write (file system, JSON, TOML, pictures), a new text's starting word
    "Text", startup errors before the window opens. A rare English slip kept word for
    word: "the centre line centre line" in `toolpath-error-medial-name`.

## State to remember (2026-11-03)

- **Released in 0.33.0** (published 2026-11-03, its guide online): styles (saved, applied,
  shown and edited), new operations that start from the selection, tabs along X or Y,
  Stock as its own item, folded operation sections, a default stock thickness, the Windows
  startup fix. Same G-code as 0.32; 0.32 can't open a project with tabs `along`, nor a
  `materials.json` with `default_thickness`.
- **Releases**: the latest published is **0.33.0** (2026-11-03). Before it 0.32.0
  (2026-11-03), 0.31.0 (2026-11-02), 0.30.0 (2026-10-31), 0.29.1 (2026-10-30), 0.28.1
  (2026-10-29), 0.27.1, 0.25.1, 0.25.0, 0.24.0, 0.23.0, 0.22.2. Tagged but never published:
  0.22.0 (withdrawn), 0.26.0, 0.26.1, 0.27.0, 0.28.0, 0.29.0. Old tags remain (`v0.3.0` on,
  the guide-only tags `guide-0.21.0-1`, `-2`, `guide-0.22.0-1`). Built by Actions from a
  `vX.Y.Z` tag: Linux and Windows x86_64 and ARM64 (AppImage, installer, portable), macOS
  universal, the guide zip, `SHA256SUMS.txt`; docs/releasing.md. Write the third-party
  notices to a temporary file first and copy it over: a stopped `cargo about >` left the
  file empty once (2026-11-03).
- **The repository is public, under the `chipload` organisation** (2026-10-17):
  github.com/chipload/chipload, the guide at chipload.github.io/chipload (guide-only fixes go
  out with a `guide-X.Y.Z-N` tag and `packaging/docs/publish-book.sh`). Actions run on every
  push to main and on tags; CI on main tests Windows but uploads no program. The `gh`
  token's resource owner is `chipload` (Contents, Actions and Pages read/write); it **can't
  comment on issues or pull requests** (403, 2026-11-02), so the user posts replies.
- **For the user to do on GitHub**: the organisation's avatar
  (`packaging/brand/png/chipload-avatar-512.png`), its name, description and website, and
  the repository's social preview (`packaging/brand/png/github-social-preview.png`), if not
  done yet.
- **Windows builds** (local fallback, without Actions): Rust (msvc) and the Build Tools are
  installed on the Windows side, checkout `C:\code\chipload`; the steps are in
  docs/releasing.md (*Without GitHub Actions*). Windows has its own fonts since 0.33 (Segoe
  UI, not fontconfig's), so text widths there differ from Linux: German ribbon names fit at
  1200 px on Windows, not on Linux.
- **Branches**: `release-0.12` (the 0.12.1 fix branch, kept for history). Local tags
  `adaptive-final` (main before trochoidal and adaptive were removed) and `python-final`.
- **Tested on a real machine:** the GRBL post, by the user on their Shapeoko (confirmed
  2026-11-03).
- **Waiting on testers**: real-controller tests of the LinuxCNC and Mach3/4 posts, inch
  G-code, and the builds for macOS, ARM64 Linux and Windows; the README asks for them.
- **Scope** (ADR 0017): a lean power tool; the yardstick is a CamBam replacement for the
  user's own router (a Shapeoko), powerful but easy.
- **Disk space** (2026-10-19): `target/` had grown to 322 GB and WSL's `ext4.vhdx` on C: to
  360 GB. Run `cargo clean` now and then, and after a Rust update; the image shrinks only
  after `wsl --manage Ubuntu --set-sparse true` (or a `diskpart` compact) on Windows.

## Next, in order

Ordered 2026-11-03 by the user's wish: "prioritize usability improvements and a
streamlined, quick and efficient workflow". The yardstick is the direction review's measure
(2026-10-28, kept private): for the usual router job, how many edits from opening the DXF
to a program you trust. It was about four per operation; with the defaults and styles of
0.33 the aim is none beyond choosing a style.

1. **Fewer clicks for the usual job.** Spec first, after a look at what each step takes
   today:
   - keys that add an operation for the selection (profile, pocket, holes, V-carve, path),
     and a key per style;
   - layer rules (docs/specs/styles.md §5): a style with a layer pattern, applied on import
     or on demand (presets.md §5, its decisions 6-8 to review then); styles of several
     operations (a profile and its pocket as one);
   - selecting by kind: a layer's shapes from its row, *select similar* (every circle of
     this size, for holes; every closed shape inside the outline, for pockets);
   - generating without F9 once the project is complete (background generation exists:
     docs/specs/background-generation.md), if it stays quick on large jobs;
   - export straight after generating: one key, the last folder and name remembered.
   (The canvas' right-click menu already adds an operation or applies a style.)
2. **DXF shapes in the Browser, and warnings that lead to their shape** (the user,
   2026-11-03: "dxf layers should be expandable to show its shapes like leafs. The report
   gives warnings about a shape in the dxf, the profile is open for example. Then a click
   on that warning should select the shape, similar to how warnings on operations is
   linked to the operation."). Today a DXF's row opens to its layers with their shape
   counts (`sources.rs`, `layers: Vec<(String, usize)>`), and a Report line points at an
   operation or a tool (`report::Target`); the drawing's warnings (open contours, invalid
   ones, the contour table from `chipload_pipeline::report::contour_messages`) point at
   nothing. Spec first: a layer opens to one row per shape (its kind and size, open or
   closed, a warning mark); a click on the row selects the shape on the canvas as a
   double-click does (`pick_alone`, the DXF stays whole otherwise); a Report line about a
   shape carries its contour (a `ReportLine` target that is an operation or a shape) and
   a click selects it and shows it; how the rows stay fast with thousands of shapes (a
   traced picture's, a large DXF's: virtual rows, or only the first N with "more…");
   pictures' and SVGs' layers alike.
3. **The walk again:** after 1 and 2, drive the usual jobs headless as the direction review
   did (a DXF part through with tabs and pockets; a V-carved sign; a sheet of copies) and
   count the edits and clicks. Each that remains becomes an item here.
4. **For the user to try** (built, not yet tried by hand; findings become items here):
   - the 0.33 Windows build: no terminal windows at startup;
   - on the router: tabs along X or Y; V-carve lifting less, with the script font (0.30);
     slot and clearing values in the tools' cutting data (the user, 2026-11-03: "it works
     quite good");
   - new operations' defaults on real jobs (through with tabs, holes first, depths from the
     last of their kind);
   - texts in Windows' own fonts (only checked on Linux);
   - SVG import with real Inkscape and Illustrator files; the exported DXF in other
     programs (LibreCAD, Inkscape, LightBurn, Fusion);
   - the 0.31 window: dockable panels, the shape library, groups, DXFs moved by their box;
   - an operation's Statistics section (lengths and lifts: do they match the router?);
   - project tools updated from the library (the offer after Save, the Report on open).
5. **Review V-carve** (safety), the last operation not reviewed (profile and pocket were,
   holes and follow path on 2026-11-02): boundaries (zero depth, depth equal to the stock,
   a bit wider or narrower than the strokes, the cone's depth, passes and step down, flat
   bottom), the checker's view, and proptest invariants (the cut stays inside the outline;
   no point deeper than the cone allows).
6. **V-carve, Ignore kinks by segment length too** (the user, 2026-11-02: "Ignore kinks
   should also have a parameter for length of adjoining segments. If the adjoining profile
   segments are very short we could probably ignore the kink"). Today a corner is ignored
   only by its angle (`ignore_kinks_deg`, docs/specs/vcarve-smooth.md). A second setting:
   a corner whose adjoining outline segments are short is ignored at any angle, as a traced
   picture's stair-steps and a font's tiny facets are. Spec first: one short side or both,
   how it combines with the angle, a default (the V-carve tolerance? the trace's
   smoothing?), a real small corner (a serif's point) still carved; the coverage rule of
   6e3ed35 still applies. Optional field, no schema bump. After the review (5).
7. **V-carve inlays** (the gap review of 2026-11-03): a plug and its pocket from one
   outline, V-carved to fit: the pocket at its depth, the plug mirrored, cut with a start
   depth and a glue gap, so the two press together and the plug's back is planed off.
   Among the most made maker jobs on a hobby router. Builds on V-carve (the medial axis,
   flat-floor passes); after the review (5). Spec first: the fields (gap, start depth, plug
   as a mirrored copy or its own setup), what the checker and the simulation show, a golden
   pair.
8. **Turning the board over** (the gap review of 2026-11-03; docs/specs/setups.md §9): a
   setup that is the same board turned over, its drawing mirrored about an axis, located
   by dowel pins or an edge, for parts machined on both faces (boxes, signs with a back
   pocket, cutting boards). Spec first: the mirror axis, how the zero follows, dowel holes
   as an operation, the setup sheet's words.
9. **Pockets: a finishing pass** (the gap review of 2026-11-03): a light last cut of the
   walls and of the floor, for fits (inlays, lids, recesses) and clean floors. Spec first:
   the allowance left by the roughing, the wall lap's direction, the floor's stepover,
   with rest passes and dogbones.
10. **First-run setup** (the gap review of 2026-11-03, and what the new defaults left): on
    the first start, ask for the machine's travel and the first tool (marked
    centre-cutting or not), so the first project has no warning but its own; today tabs
    with a tool not marked centre-cutting warn, and a machine without travel isn't
    checked against it.
11. **Laser and plasma: decide** (the gap review of 2026-11-03). CLAUDE.md promises G-code
    for "CNC routers, mills, plasma, laser", but the posts are spindle-only: no laser mode
    (power instead of RPM, M4 dynamic power, no Z), no plasma kerf or pierce delay.
    *Recommended:* narrow the claim to routers and mills until a user asks; the
    alternative is a basic laser post and mode with its own spec. The user decides.
12. **Small follow-ups:**
    - A pasted shape loses its construction flag: the clipboard carries layer and geometry
      only (`shapes_from_clipboard`, `paste_shapes` set `construction: false`).
    - Materials: import and export in the window (the command line has `material
      export`/`import`).
    - The remaining large files, split by topic as `forms/` and `models/` were, when next
      worked on: toolpath `plan.rs` (2,560 lines), `pocket.rs` (2,060), the canvas `view.rs`
      (1,940), the window's `app/mod.rs` (2,270), `app/user_library.rs` (1,470).
    - Drop `stock.origin` bottom-left/center from the format (a migration that moves the
      stock), if the user wants it (docs/specs/job-settings-layout.md).
    - Image fuzzing got slow (2026-10-13): 38 inputs a second with arcs and centre lines;
      find which pictures are slow (centre lines on noise?) and whether a user could hit it.
    - A JSON Schema for `styles.json` (`chipload schema`), as the other library files have.
13. **Later, the user's ideas for when the above is done**, each with its own spec:
    - **Setups on sheets of their own** (chosen 2026-10-30 for later; docs/specs/setups.md
      decision 2's alternative): each setup with its own coordinates, its parts placed onto
      its board, setups side by side like PrusaSlicer's plates, the active in colour, the
      others as ghosts. Spec: where a part's shapes are placed per setup, picking and
      editing in the view, migration from one shared drawing.
    - **STEP import** (2026-10-30): a STEP file seen from a chosen side (top, bottom, left,
      right, front, back), its edges as lines and arcs; faces square to the view give
      outlines at their depth (a pocket floor with its depth). Needs an ADR for the reader
      (a GPL-compatible crate that reads AP203/AP214 B-rep, or OpenCASCADE).
    - **Designs and parameters** (docs/specs/designs.md, parameters.md, a box maker with
      finger joints first): drafted 2026-10-28, on hold by the user.
    - **A full library panel**: tools and machines beside shapes (shape-library.md
      decision 12), now that panels dock.
    - **A chip-load warning per tool** (left of the lean plan's item 4; the tools' cutting
      data holds the user's own feeds without a chip-load calculation, as the user wanted):
      a range per tool and an amber warning. Ask whether it is still wanted.
    - **Chipload in the browser** (docs/specs/web.md): step 1 done 2026-10-13 (the
      libraries compile for wasm32, checked in CI); step 2, the browser app, waits (the
      user said wait, 2026-10-17).

## Ideas, not planned

Smaller follow-ups noted along the way, not in the order above:

- **Profiles:** a separate finishing feed; rest passes for profiles, if missed.
- **Pockets:** dogbones with rest passes (the finishing pass is *Next* item 9).
- **Holes** (docs/specs/holes.md §9): canned cycles G81/G83 as a post option, DXF points as
  hole centres, a clearance for a fit, dwell, chip-breaking pecks, V-bit chamfers,
  counterbores in one operation.
- **Simulation:** comparing with the design (gouges), holders and flute length; a G-code
  panel with the current line during playback.
- **Copy imported shapes** (as drawn polylines); today only drawn shapes copy.
- **A bug-report bundle.**
- **Touchpad scrolling** (offered 2026-11-03, not asked for): a preference, off by default,
  where a two-finger scroll moves the view and a pinch or Ctrl + scroll zooms; a mouse
  wheel keeps zooming. Today Space + drag pans, which the status bar now says.
- **Cutting data, not now** (docs/specs/cutting-data.md §10): faster feeds at a light
  stepover (chip thinning below about 0.3 D, finishing laps); the higher load in a ring's
  inside corners; a tool's flute length, to warn of a step down deeper than the flutes;
  the rows copied into a project's tools, so a project carries its materials' values.
- **Voronoi elsewhere:** investigate whether the segment Voronoi diagram (boostvoronoi, ADR
  0015) can make other parts faster or more robust: pocket clearing, narrow-part checks,
  distance queries (user request 2026-10-11).

## Done

### Since 0.33.1 (not released)

- **Dark corners on the panels in the light theme** (issue #14, 2026-11-04, Windows 10,
  0.33.0; seen on Linux too): egui_dock rounds the tab bars' top corners and the panels'
  bodies, and what is behind the dock showed through, dark. The dock's corners are square
  now (`layout::dock_style`). The guide's screenshots change by those pixels at the next
  renewal. Answer the issue once it is released.

### In 0.33.1 (tagged 2026-11-04)

- **Circles left behind after an explode and a move** (issue #12, 2026-11-04, the user's
  `shared_edges.dxf`): nine circles drawn twice on one layer were left out of the contours
  as duplicates, so a box round everything didn't take them and a move left them behind,
  where they became contours of their own. Chaining now lists such twins
  (`ChainResult::twins`); a click, a box or a drag takes a twin along with what it repeats,
  and an explode removes the twins it reports (explode-and-join.md §2). The G-code is
  unchanged. Answer the issue once it is released.

### In 0.33.0 (published 2026-11-03)

- **Styles** (docs/specs/styles.md, ADR 0030; the direction review's step 2): an
  operation saved by name in `styles.json` and used again on a selection (a profile style
  by the selection: holes inside, the outline outside with tabs; feeds from the tool's
  cutting data); `chipload style …`; *Save as style…*, the ribbon's Style button, *Apply
  style ▸* on the canvas, *This computer > Styles*. **Shown and edited** (§10, the user:
  "would it be possible to show and possible edit the content"): a style opens in the
  operation's own form, a draft until Save; `style show` lists its values by path,
  `style set NAME PATH=VALUE` sets them. Tried by the user: "Seems to work fine".
- **Defaults that know the job** (styles.md §2): a new profile of outlines goes outside
  and through with tabs, of holes inside and through; an outline with its holes gives two
  profiles, the holes first. New pockets, V-carves and paths take the last of their kind's
  depth, holes go through; feeds come from an operation with the same tool, else the
  tool's cutting data; a tool not known to be centre-cutting helixes or ramps. In the
  pipeline (`new_ops`) for the window and `chipload op add`. A through cut's spoilboard
  depth is a plan fact, not a warning; a helix too wide after a tool change is fitted.
- **Tabs along the grain** (docs/specs/tabs-along-grain.md, the user: "align them to the
  fibre direction"): *around*, *along X* (bottom and top edges) or *along Y* (left and
  right), half of the count on each; tabs placed by hand still win per contour. `op tabs
  --along`, the sheet, styles keep it.
- **Stock as its own item** (the user): under each setup, above *Work zero*, with its
  thickness and material beside it; the setup's form keeps name, on/off and parts. A new
  project opens on it with the cursor in the thickness; a click on the board's edge opens
  it. The setup's form lost its zero line, which repeated the Work zero item.
- **The user's window round:** Edit and Simulate each keep their own view; an operation's
  sections start folded with their summaries (one with a problem opens), labels weak and
  values brighter; submenus end in ▸ (egui's ⏵ is in none of the fonts); a default stock
  thickness for new projects (`materials.json` `default_thickness`, written only when set
  so 0.32 still reads the file; `chipload material thickness`).
- **Generate is greyed out without operations** (the user: "We should not generate if
  there are no operations").
- **Windows: no terminal windows at startup** (the user): `fc-match` from Git for Windows
  and the like ran three times; fontconfig is no longer asked on Windows. A ribbon test
  that assumed Linux's font widths was made font-independent.
- **The status bar's Edit hint starts with Space+drag moves the view** (the user had
  never come across it; touchpad panning).
- **Swedish** (the user): a profile's side *insida* / *utsida*; oversize *spel*.

### In 0.32.0 (published 2026-11-03)

- **The review findings of 2026-11-02: done 2026-11-03** (the user: "do" them, as
  suggested). Drilled through holes go through at full diameter (holes.md decision 10, the
  checker allows a drill's point); a V-bit on a follow path stops at its cone (refused
  deeper); the V-groove's width is a plain note (`Plan::facts`) and on the sheet; holes
  that don't fit say why and the smallest size; a note for holes deeper than 3 × D in one
  go, pecks of one diameter; a new Holes operation takes a fitting drill; the drill
  property test; a tool that isn't centre-cutting ramps only along a path or ring at least
  its diameter long. The guide's smallest bore said a tenth, it is a fifth (1.2 × D).

- **Project tools kept up to date with the library: done 2026-11-03**
  (docs/specs/tool-updates.md). A library tool's properties offer *Update in this project*
  for each differing copy; opening a project lists differing tools in the Report with
  *Update all from library* (also on the Tools folder's menu and page); a speed from the
  setup's material is no difference and stays; `chipload tool update`. No question on
  opening (the user agreed: a proven job cuts as it did).
- **A new project opens on its setup** (stock first; the user, 2026-11-03).
- **Deleting tools from the Browser: fixed 2026-11-03** (the user: "tools can not be
  deleted from the browser using the delete key or right click delete"). A project tool
  had only *Remove tool* in its properties: now *Delete* in its menu and the Delete key, as
  one undo step, refused (with which) while an operation or a rest or floor pass cuts with
  it; the check counted operations only, so *Remove tool* could leave a pass without its
  tool. The Delete key on a library tool or machine does as its menu's *Delete*.
- **Cutting data in the tools, slot and clearing: done 2026-11-03**
  (docs/specs/cutting-data.md, ADR 0029; the user: a pocket's first ring takes the full
  diameter, its rings 0.4 D, a profile 1 × D, "Right now it doe not feel coherent"). A
  library tool holds a row per material (and *Any material*): speed, plunge, slot feed and
  step down, clearing feed, step down and stepover; a material is a name. A pocket's
  full-width rings go down in slot steps to each clearing layer. The library converts once
  (tools.json 4, materials.json 3). For the user to try on the router.
- **The Report's first line wraps: done 2026-11-03** (the user's screenshot: the start
  hint was cut off after "(Ctr…"). It was truncated since the Report was a strip under the
  canvas (d52d988); now it wraps like the other lines.
- **Operation statistics: done 2026-11-03** (docs/specs/operation-statistics.md, the
  user's idea of a Statistics section): each operation's and pass's properties end with its
  time, cutting, plunge and rapid lengths and lifts from the last run; the Report and
  `chipload run` give the job's totals, `chipload plan` each operation's, the JSON every
  number. Not on the setup sheet, no chart, no per-tool lengths yet.
- **Russian translation, PR #11: merged 2026-11-03** (mushketer888, a CNC 3018 user):
  every text of 0.31 with Russian plural forms, its own tests and a glossary section,
  signed off. Our own fix after it: the guide's *review wanted* list leaves out Swedish
  again, and a comma.

### Earlier (0.31.0 and before)

- **Cleanup after the review of 2026-11-02** (the user: "where does the complexity lie?
  Any part that feels like a hack?"; asked for 2 then 1):
  1. **The window holds the project whole: done 2026-11-02.** `EditJob` was a `Job` with
     its operations unused plus a list of operations beside it, only to carry "side not
     chosen yet". Pipeline calls got the project without operations and their changes to
     operations were thrown away (none made any yet; the fitting board of 2026-10-30 was
     one bug of it). Now `EditJob { job, unsided }`; the marks move with their
     operations (`reorder`), pipeline results are taken whole (`adopt`).
     `Job::board_extents_with` went.
  2. **One way to say where a shape comes from: done 2026-11-02** (spec
     docs/specs/source-model.md, decisions 1-7 as recommended). `Origin` in
     `Geometry::Path`, `Selector { source, exclude }`, the window's
     `CanvasSelection::Whole` and `BoxOf::Whole`. Rust types only: the file, the scripting
     paths, the JSON Schema and the G-code are the same (all 56 corpus projects saved byte
     for byte as before).
  3. **The window's canvas tools as one enum: done 2026-11-02.** `ActiveTool` (Select,
     Draw, Modify, PlaceText, Measure) replaces four fields each cleared by hand
     (`sketch`, `modify`, `placing_text`, `measure`); `set_tool` ends the one in use and
     drops a typed value and the offset preview. Fixed on the way: Measure left an editing
     tool on (it took the clicks), Text left an editing tool on, the 3D view left Modify
     and Text on. Undo still ends drawing only.
  4. **Splitting the large files and sharing what they repeat** (the user, 2026-11-02:
     "a sound codebase to help reduce potential bugs and also make it easy to maintain and
     extend"). Order: `forms.rs`, the CLI's `main.rs`, `models.rs`.
     - **`forms.rs`: done 2026-11-02.** RestPass and FloorPass were one type twice: now
       `ClearingPass` (the file is unchanged; the JSON Schema lists one type). The pass forms
       share their rows, and the picture and text forms share their position and turn cells.
       Then split into `forms/` (widgets, operation, passes, project, machine, sources),
       none over 1,400 lines.
     - **The CLI's `main.rs`: done 2026-11-02.** Split by command group like `groups.rs`
       and `texts.rs` already were: `shapes`, `ops`, `placing` (place, zero, copies),
       `running` (run, plan, sheet, simulate, render, check); `main.rs` keeps the command
       list, dispatch, loading, import/update, get/set and inspect (1,370 lines). Saving a
       project and saying so is one `write_project` instead of five private copies.
     - **`models.rs`: done 2026-11-02.** A pure split into `models/`: project, machine,
       drawing, shapes, sources (pictures, texts), selector, operations, validate; paths
       unchanged (`chipload_job::X`, `chipload_job::models::X`). None over 1,200 lines.
  5. **The window module** (the user, 2026-11-02: before dockable panels, kept separate):
     **done 2026-11-02**, seven commits, no change in behaviour. The app's 92 fields are
     44: `panels` (layout.rs: what is open, `panel_contents(PanelKind)` apart from where a
     panel sits, `side_panels` as today's layout), `canvas_tools`, `file` and `history`,
     `toolpath` (the run state), `recovery`, `clipboard`, `prefs`, `user` (settings folder,
     library), `display`. The rest is the window's core (project, focus, selection,
     contours, scene, view) and small items.
  6. **Dockable panels**: spec docs/specs/dockable-panels.md, agreed 2026-11-02
     (decisions 1-7 as recommended), ADR 0028. **Step 1 done 2026-11-02:** egui_dock 0.21.1,
     the dock with today's layout, the canvas fixed, View > Panels and Reset layout, P hides
     every panel, the Report a panel (its warnings on its tab). **Step 2 done:**
     `layout.json` (format `chipload_job::layout`, v1 sample in tests/corpus/layout/v1). **Step 3 done:** the guide's
     screenshots renewed (`packaging/docs/screenshots.sh`). Dockable panels done.
  7. and 8. (the operations' reviews, the remaining large files) continue under *Next*.

- **A DXF or SVG selected whole on the canvas, moved by dragging** (the user, 2026-11-02:
  "imported dxfs and svgs should be able to be dragged similar to text and images. One
  click selects the whole dxf or svg and it can be dragged around and into the shape
  library"): docs/specs/dxf-whole-on-canvas.md, **done 2026-11-02**, decisions 1-4 as
  recommended. A click picks the whole DXF, a double-click one contour; its box moves, turns
  and scales it. A group's and a DXF's box stays turned (groups keep a `rotation`).

- **Explode a picture or a text into drawn shapes** (the user, 2026-11-02: "i cant explode
  and image or text to shapes only an dxf, is that by design?"):
  docs/specs/explode-pictures-and-texts.md, **done 2026-11-02**, decisions 1-5 as
  recommended. The DXF's explode generalised: the picture or text goes, ids, layers and G-code stay; `image
  explode` / `text explode`; *Explode into shapes…* in the Browser and the canvas's menu.

- **Groups of drawn shapes** (the user, 2026-11-02, about a traced picture inserted from
  the shape library: "we need to be able to group them"; docs/specs/shape-groups.md):
  **done 2026-11-02**, decisions 1-10 as recommended: `drawing.groups` (optional, no
  bump), selected as a picture is (click whole, Ctrl+click leaves out, double-click one
  shape), operations follow a group (`select.group`), Ctrl+G/Ctrl+Shift+G, `chipload
  group …`, library inserts of several shapes come in as a group.

- **Shape library in the right panel** (the user, 2026-11-02: "a tab in the right panel
  where I can drag and drop shapes from the library to the canvas … and also the other way
  around"; "Each item shall have a preview and a name"): docs/specs/shape-library.md §5.1,
  **done 2026-11-02**, decisions 9-14 as recommended: entries (`.chipload-shapes`, v1),
  groups as folders, `shape save/insert/library`, the Shapes tab with drag and drop both
  ways. The guide's screenshots were renewed for 0.31.0.

- **V-carve step down** (the user, 2026-11-02: "there should be an option to use ordinary
  step down as well"; docs/specs/vcarve-step-down.md): **done 2026-11-02**, all decisions
  as recommended. *Depth passes*: Equal load (passes) or Step down (mm, as a pocket);
  optional `step_down`, no schema bump.

- **Smoother V-carve** (the user, 2026-11-01: "a lot of small up and down movement just
  because the path is not entirely smooth … prune voronoi branches if the angle is more
  than say 60°"; docs/specs/vcarve-smooth.md): **done 2026-11-01**, all decisions as
  recommended. The depth smoothed within half the tolerance (always); *Ignore kinks*, 20°
  for new V-carves, 0° in older projects. Z reversals 326 → 172 on the script example.
  Only a branch the rest covers goes (a traced logo at 20° had lost a leg of its "H").
  Tried by the user on the traced logo (2026-11-02): "now it works, 20 or 30 deg is a good
  value"; the default stays 20°.

- **Image window** (the user, 2026-11-01: the traced paths had disappeared; the picture
  should look like V-carved wood): **done 2026-11-01.** Light oak and dark brown instead of
  white and near black; the traced lines, outlines too (left out on 2026-10-14 for hiding
  the detail), thin and blue, with *Show the traced lines* to hide them.

- **V-carve lifts less** (the user, 2026-10-31, testing with Edwardian Script: "the bit go
  up and down doing seemingly nothing"; docs/specs/vcarve-fewer-lifts.md): **done
  2026-10-31**, all decisions as recommended, in 0.30.0. Corner branches under 5 mm out
  and back, pass pieces within 10 mm joined through the groove; the carve the same (pixel
  for pixel in the simulation), about half the lifts. To try on the router with the
  user's script font.

- **Delete from the canvas** (the user, 2026-10-31: "text is not deletable from the
  canvas… all shapes should be deletable via canvas"): **done 2026-10-31.** Del (and
  right-click > *Delete*) removes a text, a picture or a DXF selected as a whole, as its
  *Remove* does; a box selection takes drawn shapes and the texts of selected letters, as
  one undo step. A single shape of an imported DXF or traced picture still stays (it would
  come back with the file).
  The same day (the user): Del also deletes an operation or rest pass picked in the
  Browser, and a text opened there: Del takes what Properties shows. Parts and setups stay
  right-click only (they take their operations with them).

- **Rulers on the canvas** (the user, 2026-10-31: "a ruler so I can see the scale of axis
  directly"; docs/specs/rulers.md): **done 2026-10-31**, drawing coordinates (the user's
  choice), ticks on the grid, the cursor marked, *View > Show rulers*.

- **Frames and a shape library for signs** (the user, 2026-10-31: "a library/catalog of
  frames where the user can quickly insert a frame and also save frames or shapes for
  future use"; docs/specs/shape-library.md): agreed 2026-10-31, all decisions as
  recommended, "we will iterate a bit on it". **Step 1, frames, done 2026-10-31:** seven
  frames round the selection or to a size, 0–3 border lines, *Draw > Frame…* and
  `chipload frame add`/`kinds`, golden v35_sign_frame, guide page *Signs and frames*.
  Step 2, the shape library, done 2026-11-02 (*Shape library in the right panel*,
  above).

- **Texts in the computer's fonts** (the user, 2026-10-31: "allow to use fonts from the
  computer when creating texts"; docs/specs/system-fonts.md, ADR 0027): **done
  2026-10-31**, all decisions as recommended. Installed fonts and the settings folder's
  `fonts/` (fontdb, not in the browser build), by full name; only the name is stored. A
  text without its font keeps its letters and cuts; its words, size, alignment and
  spacing are locked with a note, moving, turning and another font work. To try on
  Windows with its fonts (only checked on Linux).

- **Shakedown findings** (the user on the router, 2026-10-28; the checklist is kept
  private, outside the repository):
  1. The stock should sit centred on the drawing, sticking out equally all round. Done
     2026-10-28 (docs/specs/stock-around-drawing.md, schema 33): new projects get a stock
     centred on the drawing that fits it with 10 mm, a fixed size stays possible, and the
     report gives the cut's outline (the smallest board that holds the job).
  2. With an operation open, a shape couldn't be picked for editing: Properties kept the
     operation. Fixed 2026-10-28: a click on a shape closes the open operation; the
     selection stays for its *Use selection*.
  3. A new project showed the problem "Job has no operations" straight away. Fixed
     2026-10-28: the window leaves it out; Generate still says so if pressed.
  4. Properties showed two things (the selection above Job settings), which confused.
     Fixed 2026-10-28: one thing at a time, the Browser's item or the canvas pick
     (`Focus::Selection`); no Browser row is highlighted while the pick is shown.
  5. An operation should open by clicking its number (and circle) on the toolpath. Done
     2026-10-28 (`Scene::op_label_at`).
  6. *Fit the drawing* was a button pushed off a narrow panel, and should be the default.
     Done 2026-10-28: a box at the top of Stock with the margin beside it; projects
     started empty fit the drawing too.
  7. The array tools couldn't be finished: their Create was under Selection, hidden while
     another item was open. Fixed 2026-10-28: starting one shows Selection, its form has
     *Cancel*, and the ribbon button ends the active tool.
  8. A DXF or SVG couldn't be added to a project without a drawing file yet: *Add DXF*
     was off. Fixed 2026-10-28: any open project takes one.
  9. Dragging a polyline's line should move that segment; a new point and a bulge should
     need modifier keys. Done 2026-10-28: plain drag moves the segment, Ctrl inserts a
     point, Shift bends (`SegmentDrag`).
  10. A text clicked in the view selects its letters' shapes, not the text: its
     properties (words, font, height) open only from the Browser (the user, 2026-10-30).
     Fixed 2026-10-30: a click on a letter opens the text, as its Browser row does and
     the guide said; the text stays selected for *Use selection*, Ctrl+click picks letters.
  11. A library machine couldn't be changed (the user, 2026-10-31). Fixed 2026-10-31: its
     form never wrote back the G-code units and tool changes, so changing them did nothing
     and *Save* stayed greyed (`take_machine_values` names every field).
  12. Choosing another machine reset the work zero (the user, 2026-10-31). Fixed
     2026-10-31 (docs/specs/linked-machine.md decision 6): using a profile in an open
     project, in the window or with `chipload machine use`, takes its machine values only
     and keeps the project's safe Z, work zero and max overcut, as *Update from library*
     does; a new project still starts from all of them.
  13. A V-carve in passes went back and forth over the whole carving (the user, after the
     test cut 2026-10-31). Fixed 2026-10-31 (docs/specs/v-carve.md decision 13): each area
     gets all its passes, centre line and walls, before the nearest next one.
  14. The Simulate button didn't stand out (the user, 2026-10-31). Done 2026-10-31
     (docs/specs/edit-and-simulate.md decision 10): an **Edit | Simulate** switch first in
     the navigation bar, in an accent outline with the mode's half filled, and an accent
     frame round the canvas in Simulate.
  15. The machine's properties were crammed and hard to read (the user, 2026-10-31). Done
     2026-10-31 (docs/specs/linked-machine.md decision 7): one **Profile** dropdown naming
     the linked profile, one status line with only the buttons that apply, *Save as new…*
     and *Show in library* behind ⋯, the greyed values under **From the profile**. The same
     for project tools.

- **Drawing in Chipload, self-sufficient** (approved 2026-10-27,
  docs/specs/drawing-roadmap.md; ADR 0017 amended, ADR 0023): phase 1, precise input and
  the basic tools (docs/specs/drawing-input.md), **done and pushed 2026-10-27**: the
  `chipload-draw` crate with more snaps; keys, typed values and angle steps; box
  selection and moving with the mouse; arc and slot tools; polygons (schema 30); lengths
  on the canvas. Not tried by hand in the running app beyond screenshots; the guide's
  screenshots renewed for 0.28.0.
  Phase 2, editing (docs/specs/drawing-editing.md), **done 2026-10-28, not pushed**:
  offset, fillet and chamfer, trim, extend and break, union, subtract and intersect,
  grid and polar arrays, align and distribute, each on the command line and in the
  window; golden v30_drawn_edited drawn with these commands alone. Next: phase 3
  (SVG import through `usvg`, added 2026-10-27; spec approved 2026-10-28,
  docs/specs/svg-import.md, ADR 0024: **done 2026-10-28, not pushed**: the reader crate,
  schema 31, the command line and the window; follow-ups: try it by hand with real
  Inkscape and Illustrator files), 4 (designs, a box maker with finger
  joints first, extensible to more designs, docs/specs/designs.md, and parameters,
  docs/specs/parameters.md: both drafted 2026-10-28 and **on hold** by the user the same
  day), 5 (DXF export and construction geometry: spec approved 2026-10-28,
  docs/specs/dxf-export.md; DXF export and construction geometry (schema 32) done
  2026-10-28, not pushed; follow-ups: try the exported DXF in other programs (LibreCAD,
  Inkscape, LightBurn, Fusion), keep the construction flag through a clipboard paste).
  **Next (the user, 2026-10-28): step back and think about the app's direction, the
  workflow now and what we want in the future, before more features.** Review written
  (kept private, outside the repository; yardstick: a CamBam replacement for the user's own
  router, powerful but easy; recommends A, the daily driver: a shakedown on the router,
  styles/presets, materials with feeds, defaults that know the job, first-run setup). The
  user chose A and started with the shakedown on the router (above).
  **Materials first, then styles** (the user, 2026-10-28: simple, your own numbers, no
  manufacturers' charts): built 2026-10-28 with the recommended decisions for the user to
  try (docs/specs/materials.md §10); then the same day a material per operation (a list
  under the tool) and no thickness, as the user asked. Open: the user's verdict;
  import/export of materials in the window. Then styles (docs/specs/presets.md).
  **Job settings, clearer** (docs/specs/job-settings-layout.md, 2026-10-28): four sections
  (stock, zero, material, machine), stock size "outline + margin" or "fixed, centred", a
  default material; and §5 (2026-10-29), a *Through* box per profile, pocket, path and
  holes whose end depth follows the thickness (`through`, optional, no bump); built. Open:
  dropping `stock.origin` bottom-left/center from the format (a migration that moves the
  stock) if the user wants it.
  **Work zero as a marker** (docs/specs/work-zero-marker.md, 2026-10-29, the user's idea):
  a Browser row, a handle in the view that is picked and dragged with snapping, stock
  corner buttons in Properties, a `zero` command; new projects and machine profiles start
  at the stock's bottom-left corner; built. Later: **setups** (a board with its own stock
  and zero, holding parts), raised by the user as a zero per part. After the first test cut
  (2026-10-29): the drawing origin as ghost axes once the zero is elsewhere; built.
  **One part's G-code** (docs/specs/part-export.md, 2026-10-29): *Export G-code of this
  part…* on a part's right-click menu, `--part` on run/sheet/plan; built.
  **A pocket stays down between its depth passes** (docs/specs/pocket-stay-down.md,
  2026-10-29, the user missed *keep tool down* for deep holes): lifts 0.5 mm and feeds
  back where checked safe; built. Fixed the same day: letting go of a dragged tab
  or start point no longer moves Properties to the shape selection. A tab or start dropped where the
  planner refuses it goes back with the reason, the markers staying (no Reset needed). The chip load
  has its own row under the feed; a project's tool shows its flutes in Properties. While trimming, extending or
  breaking, a click on empty space deselects, and with nothing selected ends the tool; a trim
  joins what is left with drawn shapes whose ends meet it (a slot trimmed by a line is one closed
  shape).
  CI runs on every push to main again since 2026-10-29 (by hand only from 2026-10-27, which let
  platform-dependent pocket G-code through to the 0.28.0 release build).

- **Edit and Simulate** (2026-10-30, the user: box selection didn't work after the
  simulation was shown, then "a distinct simulation switch"; docs/specs/edit-and-simulate.md):
  two canvas modes, Edit (projects open in it: drawing, selecting, handles) and Simulate
  (**Simulate** on the navigation bar, View > Simulate, F6: the shaded stock and playback, a
  drag pans or orbits, a click opens an operation by its number). A drawing tool or a
  Browser selection goes back to Edit, which always shows the drawing. The simulation is
  computed only in Simulate and kept while in Edit. Only edits that can change the cut make
  a new toolpath (`chipload_pipeline::cut_inputs`): a shape no operation selects doesn't,
  unless it grows a fitting board or lies off the stock. To try by hand.
- **Update notice** (2026-10-30, the user: "could we also add an update notification when
  there are new releases available at GitHub?"; docs/specs/update-notice.md, ADR 0026):
  asked once; when on, a daily check at start of GitHub's latest published release (one
  HTTPS request, nothing about the user sent); *Chipload X is available* in the status
  bar and under Help with *What's new…* and *Skip this version*; *Help > Check for new
  versions now*; the setting in Preferences. `ureq` + `rustls`/`ring` with the system's
  certificates (`rustls-native-certs`), in the window only. Checked against the real
  GitHub by hand (`github_answers`, ignored).
- **Windows installer** (2026-10-30, the user: "could we add a windows installer so we can
  open chipload files with a double click?"; docs/specs/windows-installer.md, ADR 0025):
  `chipload-X.Y.Z-windows-<arch>-setup.exe`, NSIS built on Linux, per user: the app and
  the command line, `.chipload` opened with a double-click, Chipload in *Open with* for
  DXF and SVG, Start menu, uninstaller that keeps the settings. The release workflow
  builds both and tests a silent install and uninstall on Windows. To try by hand before
  the first release with it (spec §7). The Windows CLI zips are dropped (the user: "drop
  cli"): the installer carries `chipload.exe`; Linux and macOS keep their archives.
- **V-carve passes of equal load** (2026-10-30, the user: "each stepdown will cut the entire
  length of the v-bit", then "final depth: 5 mm, passes: 6"; docs/specs/v-carve.md decision
  12, schema 35): a V-carve has a number of passes, 1 to 100, in place of its step down;
  each removes the same cross-section, pass k of n at the depth × √(k / n), so the steps get
  smaller on the way down, and the window shows the depths under the field. Migration 35:
  as many passes as the step down took (⌈depth ÷ step⌉), so no pass is heavier than the
  old last one; the depths change. To try on the router.
- **Setups** (2026-10-30, docs/specs/setups.md, schema 34; the user: "we will probably
  need to iterate on this"): a project's boards, each with its stock, work zero and parts,
  each a program of its own (`-S1`, `-S2`), with its own sheet. Command line `setup …`,
  `--setup`; in the window *Setups* in the Browser, the active one in full colour and the
  others as ghosts (as PrusaSlicer shows plates). To iterate: export writes the active
  setup only; the Browser keeps *Parts* as one list (parts say their setup); turning the
  board over is a later spec (§9). Fixed 2026-10-30: with two setups, a board that fits
  its parts showed the whole drawing's size in its Stock section, and the zero's form and
  handle placed the zero on that board (the window's edited copy has no operations);
  `plan` listed the other setups' parts and chip loads, now only the planned setup's.
- **V-carving wide strokes** (2026-10-30, docs/specs/vcarve-wide-strokes.md, the user's
  request): where a stroke is too wide for the depth, the V-bit cuts walls along the outline
  instead of a capped centre line; an optional floor pass (an end mill, before the V-carve)
  clears the floor, the V-bit cleans up where it can't reach, and a note gives the share
  cleared and the largest end mill that would clear it all, with the library's that do.
  Command line `op add-floor`/`remove-floor`; Browser leaf, *Add floor pass ▸*, its form.
  To try on the router: does it feel right, not obtrusive but convenient. Not built: an
  action on the floor-left warning. Fixed 2026-10-30: the Browser's tool-change line
  follows the cut order, "tool change → T2 → T1" above a V-carve with its floor pass.
- **SVG fuzzing, first run** (2026-10-30): an elliptic arc reaching x = 3e62 ran usvg out
  of memory (it split the arc into ten billion curves). Numbers beyond 1e9 in an element's
  geometry are refused before usvg reads the file (docs/specs/svg-import.md §9). "Cannot
  read DXF" said DXF for an SVG too; now "Cannot read FILE". The second run timed out on a
  150 m contour: the fold check sampled whole edges; it now samples only where an edge
  passes the other's box (17 s to 38 ms, the same answers).
- **Texts for any drawing file** (2026-10-30, a follow-up of SVG import): the window's
  texts that said DXF where an SVG works too say "drawing file", "the file" or "DXF or
  SVG" in all six languages; a selection names "drawing 2", not "DXF 2"; an imported
  shape's entity no longer says "DXF" before an SVG's.
- **A move more on macOS** (2026-10-30, found 2026-10-24): a ramp over a tab ended 2e-15 mm
  above its depth and a plunge of that length followed, on some platforms only. A vertical
  step under `Z_NOISE_MM` is no move now; the G-code was the same everywhere already.
- **Traced loops "crossing" on a tiny picture** (2026-10-30, found 2026-10-28): the loops
  never crossed; two edges on one line, apart, looked crossing to the test's orientation
  check through a last-bit error. The check now wants their boxes to overlap; a fixed test
  and the proptest seed keep the case.
- **Platform-proof pocket choices** (2026-10-30): the pocket's "nearest" choices (cut order,
  ring starts, stored start points, region order, helix room) take distances within the
  chaining tolerance as a tie, as the lead's start did since 0.28.1 (docs/specs/pocket.md).
- **The legend and the navigation bar no longer overlap** (2026-10-29, seen 2026-10-28 in
  German at 1200 × 800): when even the compact bar has no room beside the legend, the bar
  is centred and the legend stands on it; in a wide canvas the bar keeps to its right as
  before. Layout test `the_legend_stays_clear_of_the_navigation_bar`.
- **Surface property test that failed on CI** (2026-10-29, found 2026-10-27 on Windows): a
  test bug, not a code bug. `surface_moves` runs a stock that is square within the chaining
  tolerance along X; the test took plain `w >= h`, so for a stock 0.00012 mm taller than
  wide it checked the descents on the wrong axis. The test now uses the same rule, the CI
  seed is kept in the regressions file, and 20 000 cases pass. The tool always goes down at
  its passes' ends beside the stock, whichever way they run.
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
