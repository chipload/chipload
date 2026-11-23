# TODO

What's planned. Each item gets a spec in `docs/specs/` that is reviewed before it's built
(see `CLAUDE.md`). Pruned 2026-11-20 before 0.39.0 (the user: "prune the todo"): what was
done is in git and in the releases' notes on GitHub, no longer listed here.

## Release 0.40.0 "Signs" (prepared 2026-11-23)

- **State:** the user's findings from testing fixed (2026-11-23). Version 0.40.0, the
  third-party notices regenerated, the guide's screenshots renewed, the checks run (tests,
  clippy, fmt, `cargo deny`, the libraries for wasm32). Main pushed; **the tag is the
  user's** (`git tag -a v0.40.0 -m "Chipload 0.40.0"`, docs/releasing.md).
  Run the tests on the Windows clone before tagging (new window tests: New's start
  choices, a polyline closed by another). Then the draft release, then the guide
  (`packaging/docs/publish-book.sh v0.40.0`).
- **Its notes** (draft, to paste into the GitHub release and edit):

  > **Chipload 0.40.0: signs**
  >
  > **Signs**
  > - **Signs from a list:** *New sign… › Signs: From a list* makes the same sign for
  >   every line, `|` starting a sign's second line (`ANNA BERG | ROOM 12`). With a fixed
  >   size every sign gets the same letter height, so they match; all go in one part, cut
  >   together. `chipload sign add --list names.txt`.
  > - **A fixed size:** *Size: Fixed* takes the sign's outer size and finds the largest
  >   letters that fit (`sign add --size 300x100`).
  > - **Round words** on a round frame, and **text on an arc** for any text.
  > - **Single-line text:** the Relief SingleLine font gives open strokes for a follow path
  >   with a V-bit or engraver: quick lettering, one pass.
  > - *New sign…* on the start page; the preview is drawn alongside while you type, and a
  >   script font's sign is made in a fraction of the time it took.
  >
  > **Drawing**
  > - **Fillet and chamfer between two shapes**, and at a polyline's corner beside an arc;
  >   no need to select first.
  > - **Arcs while drawing a polyline:** A switches to tangent arcs, L back to lines.
  > - **Scale by a length** (two clicks and the new length) and **stretch** in X or Y: side
  >   handles on the selection's box, *Keep proportions* in the form.
  >
  > **Nesting**
  > - A part's count in the Nest window covers every board of its stock: raising it fills
  >   the board that took the overflow before a new one is made.
  >
  > **Machines**
  > - New post-processors: **MASSO** and **Centroid Acorn**, written from their manuals and
  >   not yet tried on a machine (reports welcome). Each post names the controllers it is
  >   for.
  > - LinuxCNC in inches: `G64 P` is now given in inches.
  >
  > **V-carve** (from a safety review)
  > - Passes share the depth actually carved, not a generous end depth (which turned three
  >   passes into one full-depth pass); the depth checked against the stock is the depth
  >   carved; a new V-carve starts at its bit's cone depth; a loop (an O) starts at its
  >   shallowest point.
  >
  > **Also**
  > - *New* with a project open offers the start screen's choices.
  > - `chipload op remove`, `op duplicate` and `op move`.
  >
  > **Compatibility:** projects with a radius at a corner beside an arc (schema 43) can't
  > be opened by 0.39 or older. V-carves' G-code changes (pass levels and where loops
  > begin; the deepest Z is the same), as does LinuxCNC's in inches (`G64 P`); every other
  > older project gives the same G-code.

## In progress

- **Translations** (docs/specs/translations.md, ADR 0022): the app, its messages and the
  setup sheet in English, German, French, Spanish, Italian, Swedish and Russian (machine-made
  from `crates/chipload-i18n/GLOSSARY.md`; Russian since PR #11). Left:
  - **Native review:** the user reviews Swedish as they use it; German, French, Spanish,
    Italian and Russian are marked *review wanted* (`Language::reviewed`). The translators'
    open suggestions are at the end of the glossary. Everything since 0.31 is machine-made
    in every language, Swedish too. Mark Swedish reviewed once the user has read it.
  - **Still English on purpose:** a DXF's issues as stored in the project (a schema change;
    left unless users ask, 2026-10-25), migration reasons, a library file's rule
    violations, other libraries' texts, a new text's starting word "Text", startup errors
    before the window opens.

## State to remember

- **Releases:** the latest published is 0.39.0 "Drawing" (2026-11-20, its guide online),
  then 0.38.0 (2026-11-18). 0.39.0's tag was moved three times before it built: Windows-only
  test failures, fixed in the tests (a temporary folder made again too soon; clicks at
  pixel positions that Windows' fonts move).
  Built by Actions from a `vX.Y.Z` tag: Linux and Windows x86_64 and ARM64, macOS
  universal, the guide zip, `SHA256SUMS.txt` (docs/releasing.md). Write the third-party
  notices to a temporary file first and copy it over (a stopped `cargo about >` once left
  it empty).
- **The repository is public**, github.com/chipload/chipload, the guide at
  chipload.github.io/chipload (guide-only fixes: a `guide-X.Y.Z-N` tag and
  `packaging/docs/publish-book.sh`). The `gh` token can't comment on issues or pull
  requests; the user posts replies.
- **Windows builds without Actions:** Rust (msvc) and the Build Tools on the Windows side,
  checkout `C:\code\chipload` (docs/releasing.md, *Without GitHub Actions*). Windows text
  widths differ from Linux (Segoe UI).
- **Tested on a real machine:** the GRBL post, on the user's Shapeoko. **Waiting on
  testers:** the LinuxCNC and Mach3/4 posts, inch G-code, the macOS, ARM64 Linux and
  Windows builds.
- **Scope** (ADR 0017): a lean power tool, a CamBam replacement for the user's own router,
  powerful but easy; nesting came into scope 2026-11-16.
- **Tests in the sandbox:** process numbers repeat between runs, so a test's temporary
  folder named after the process must be emptied first (done for the CLI tests and the New
  project dialog's).
- **Disk space:** `target/` grows large; `cargo clean` now and then and after a Rust
  update.

## Next, in order

0. **From the capability review** (2026-11-22, the user: "Do 1, 2 and 4"), specs approved:
   - ~~single-line text~~ built 2026-11-22 (docs/specs/single-line-text.md), pushed, for
     the user to try;
   - ~~more post-processors~~ built 2026-11-22 (docs/specs/more-posts.md): MASSO, Centroid
     Acorn, controllers named; UCCNC waits for a manual that says how it reads I and J and
     its units (§8);
   - ~~signs from a list~~ built 2026-11-22 (docs/specs/batch-signs.md), pushed; from the
     user's testing 2026-11-23: the preview is drawn on a thread of its own, so typing never
     waits (a list's only draws the signs), the list's format stays shown under it, and long lists are quick (a frame's
     box no longer chains the whole drawing; nesting tests boxes before polylines: 100
     signs 15 s → 1 s);
   - ~~*New* with a project open shows the start screen's choices~~ (the user, 2026-11-23),
     cancellable back to the project; New sign after *Don't save* now opens the sign;
   - ~~a polyline closed by another lost its handles~~ (the user, 2026-11-23): the clicked
     shape of a chained outline is the one edited;
   - ~~*Make sign* on a thread of its own~~ (2026-11-23): the dialog says it is being made,
     the window never stops; refused, the dialog stays open;
   - ~~nesting overflow after raising counts step by step~~ (the user, 2026-11-23): a
     part's count covers every board of its stock, so what went to Setup 2 is counted and
     Setup 2 fills before a new board is made;
   - **Layers for drawn shapes** (the user, 2026-11-23): layers feel strange for our own
     shapes; rather a flat list, an exploded DXF becoming shapes. To talk about.
   - ~~`op remove`, `op duplicate`, `op move`~~ on the command line (found missing while
     building; the window had them), 2026-11-22, the rules in `chipload_pipeline::operations`.
1. **Drawing, step 2** (the review of the drawing tools, 2026-11-18; docs/specs/
   drawing-improvements.md), in this order:
   - ~~fillet and chamfer at line-arc corners and between two shapes~~ built 2026-11-21
     (docs/specs/fillet-between-shapes.md, schema 43), pushed, for the user to try;
   - ~~arcs while drawing a polyline~~ built 2026-11-21 (docs/specs/polyline-tangent-arcs.md),
     pushed, for the user to try;
   - ~~text on an arc~~ built 2026-11-21 (docs/specs/text-on-arc.md, the sign wizard's
     round words too), pushed, for the user to try;
   - ~~scale by reference and stretching in X and Y~~ built 2026-11-22
     (docs/specs/scale-and-stretch.md), pushed;
   - spline to polyline (explode), joining open splines, their end tangents;
   - extending arc ends along their circle;
   - an arc's form by centre, radius and angles when a polyline is one arc;
   - snaps: a "from" base point, tracking along a line, a line tangent to two circles;
   - lengths on the canvas for arcs, angles and polygon sides;
   - copies along a path;
   - named values (the drawing roadmap's phase 4; schema).
2. **DXF shapes in the Browser, and warnings that lead to their shape** (the user,
   2026-11-03): a layer opens to one row per shape (kind, size, open or closed, a warning
   mark); a click selects it on the canvas; a Report line about a shape selects it; rows
   that stay fast with thousands of shapes. Spec first.
3. **The walk again:** drive the usual jobs headless (a DXF part with tabs and pockets, a
   V-carved sign, a sheet of instances; now also one drawn from scratch) and count the
   edits and clicks; each that remains becomes an item here.
4. **V-carve: reviewed 2026-11-22** (safety). Sound: every centre-line move and wall is
   verified against the real outline before use; validation holds at every boundary tried
   (zero depth, start at or below end, passes 0/100+, step down, tip >= diameter, angle 0 and
   near 180, depth past the overcut); the cone caps the depth; the floor pass is a checked
   pocket; rapids sideways only at safe Z. New: a property test of the moves as written
   (joined through grooves, in passes, flat tips, small bits, plugs), not only the planned
   paths: no gouge in 1,000 random cases. Its four findings fixed the same day (the user:
   "Go ahead"): the passes share the depth the carve goes to, not the end depth (a fact
   says so); the depth checked against the stock is the planned deepest (with a floor pass
   on wide strokes, the end depth), no longer the end depth; a new V-carve starts at the
   bit's cone depth, at most half the stock; a loop of the centre line starts at its
   shallowest point (a ring of even width still goes in at nearly full depth: a ramp
   along the loop could come later).
5. **V-carve, Ignore kinks by segment length too** (the user, 2026-11-02): a corner whose
   adjoining segments are short ignored at any angle. Spec first (the review is done).
6. **First-run setup:** on the first start, ask for the machine's travel and the first tool.
   A new project without a library now has a travel and a centre-cutting tool (2026-11-20),
   so this is about the user's own values, not the warnings.
7. **Laser and plasma: parked** (2026-11-22, the user). The claim is dropped from CLAUDE.md;
   docs/specs/laser-and-plasma.md is kept for when it comes back.
8. **Leftovers of what was built** (each small; do when it comes up):
   - drawing: the dashed line from a rounded corner's handle to its arc
     (polyline-radii.md §4); a true SPLINE in DXF export (an R2000 writer) or control-point
     splines, if wanted;
   - instances: snapping to the board's corners and to other instances while moved or
     dropped (drawing-and-setup-spaces.md §4); the "too large for any board" mark kept
     only for the session (nest-overflow-and-all.md §6);
   - nesting (nesting.md §11): small parts in holes, `dxf export --as-cut`, a front and back
     nested together; an odd nest on a tall board, waiting for the user's project;
   - pieces: dragging a seam in the view, `plan --piece`;
   - flutes: a marker at each flute's start end; moulding: a step in the relief wizard,
     different ends, linking passes;
   - reliefs: a time estimate in the wizard, dropping a file on it, the board's size in
     step 3; simplifying a large model; an end-mill pass for a wide flat;
   - signs: a second sign beside the first.
9. **Small follow-ups:**
   - Large files and functions (aim: files under 1,000 lines, one job per function). Left
     over 1,000 lines: `app/compass.rs`, `simulate.rs`, `app/modify.rs`, `app/menu.rs`,
     `new_ops.rs`, `pieces.rs`, `relief.rs`, the job's `models/project.rs` and
     `models/shapes.rs`, the checker's `lib.rs`, `app/sources.rs`, `vcarve.rs`, `lap.rs`,
     `pipeline/nest.rs`, and the test files `cli/tests/cli.rs`, `job/tests/jobs.rs`,
     `pipeline/tests/pipeline.rs`, `toolpath/tests/pocket.rs`. How: code moved as is (one
     commit per file, golden files untouched); a split file re-exports its public paths;
     the window's methods in `impl ChiploadApp` blocks by topic; tests in `foo/tests.rs`;
     GUI texts stay literal `t!(ui, "id")` calls.
   - Drop `stock.origin` bottom-left/center from the format, if the user wants it
     (docs/specs/job-settings-layout.md).
   - Image fuzzing is slow (38 inputs a second with arcs and centre lines): find which
     pictures are slow and whether a user could hit it.
   - A JSON Schema for `styles.json`, as the other library files have.
   - The window test `the_tabs_switch_spaces_and_a_double_click_opens_an_instances_part`
     is skipped on Windows: its clicks at pixel positions missed there in the release
     build (Windows' fonts lay the headless window out otherwise; 2026-11-20). Make it
     independent of the layout (click the tabs through their accessibility nodes, find a
     clear point on the instance from the window's own rects), then run it there again.
10. **Later, the user's ideas**, each with its own spec:
    - **STEP import** (2026-10-30): a STEP file seen from a chosen side, its edges as lines
      and arcs, faces square to the view as outlines at their depth. Needs an ADR for the
      reader.
    - **Designs and parameters** (docs/specs/designs.md, parameters.md; a box maker with
      finger joints first): on hold by the user.
    - **A full library panel:** tools and machines beside shapes, now that panels dock.
    - **A chip-load warning per tool:** a range per tool and an amber warning. Ask whether
      it is still wanted.
    - **Chipload in the browser** (docs/specs/web.md): the libraries compile for wasm32
      (checked in CI); the browser app waits (the user, 2026-10-17).

## Ideas, not planned

- **Profiles:** a separate finishing feed; rest passes for profiles, if missed.
- **Pockets:** dogbones with rest passes; a finishing pass with a tool of its own
  (pocket-finish.md decision 1), if wanted.
- **Holes** (docs/specs/holes.md §9): canned cycles G81/G83 as a post option, DXF points as
  hole centres, a clearance for a fit, dwell, chip-breaking pecks, V-bit chamfers,
  counterbores in one operation.
- **Simulation:** comparing with the design (gouges), holders and flute length; a G-code
  panel with the current line during playback.
- **Copy imported shapes** (as drawn polylines); today only drawn shapes copy.
- **A traced outline that crosses itself** (the user, 2026-11-15: a prism of a traced
  picture was refused; not reproduced yet, the user will): a tracer bug to fix with the
  picture; perhaps skip crossing outlines with a warning instead of refusing the operation.
- **A bug-report bundle.**
- **A fixed pin station for two-sided batch work** (the user, 2026-11-10): pin diameter,
  spacing and direction in the machine profile; *drilled for this job* or *the machine's
  station* in *Two-sided…*. Spec when someone works that way.
- **Touchpad scrolling** (offered, not asked for): a preference where a two-finger scroll
  moves the view and a pinch zooms.
- **Cutting data, not now** (docs/specs/cutting-data.md §10): faster feeds at a light
  stepover, the higher load in a ring's inside corners, a tool's flute length, the rows
  copied into a project's tools.
- **Voronoi elsewhere:** whether the segment Voronoi diagram (ADR 0015) can make pocket
  clearing, narrow-part checks or distance queries faster or more robust.
- **Sliding tiles:** one long board slid between tiles on pins (docs/specs/sliding-tiles.md,
  parked).
