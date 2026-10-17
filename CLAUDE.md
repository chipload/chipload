# CLAUDE.md

Project: **Chipload**, a cross-platform 2.5D CAM application, written in Rust.
Input: DXF. Output: G-code for CNC routers, mills, plasma, laser.
This is open-source software (**GPL-3.0-or-later**, copyright Gustav Näslund) that drives physical machines. Correctness and safety outrank speed and cleverness.

The original Python implementation was the reference during the port and has been removed (ADR 0009). It is preserved at the git tag `python-final`.

## Crates (`crates/`, one per layer)

| Crate | Role |
|---|---|
| `chipload-core` | primitives, chaining, nesting, line-and-arc fitting, tolerances. No I/O, no GUI, no other workspace crate. |
| `chipload-job` | project file models (schema v29: embedded drawings, several DXFs each with its placement (move, turn, scale), hidden groups, drawn shapes, depth ranges, clearance, optional grid of copies, V-carve operations and V-bit angle/tip, a tool per operation and the machine's tool change setting, rounded ring corners, a copied user post-processor, tab flank angles, follow-path operations, holes operations, dogbone and T-bone corners, rest pockets, images traced into shapes or centre lines, with arcs, parts each with its operations, copies and on/off, cut part by part, tools and machine linked to library entries by uid, DXFs and pictures turned by any angle, rounded rectangle corners, texts (optional `drawing.texts`, no bump); since v26 no clearing settings: links where safe and rounded rings always; since v27 profiles always spiral: a ramp angle instead of an entry, and no lead; since v28 the machine's G-code units, mm or inch; since v29 surface operations), migrations, load/save; the user's tool library and machine profiles (`library`, copied into projects, ADR 0014), app settings, recovery notes; `save_job` keeps the previous file as `.bak` |
| `chipload-dxf` | DXF import into shapes, shapes into contours, the `ImportReport` |
| `chipload-image` | pictures (PNG, JPEG, BMP) into black and white and traced loops for V-carving, with arcs (docs/specs/image-trace.md, image-arcs-centrelines.md); core only, no file I/O |
| `chipload-text` | text into closed outlines with arcs, in 17 built-in fonts: GNU FreeFont and ten Apache-2.0 fonts (docs/specs/text.md, more-fonts.md, ADR 0020, 0021); core only, no file I/O |
| `chipload-toolpath` | selection, offsets, profiles (spiral laps, ramped tabs, finishing laps, onion skins, dogbones), tabs, pockets (with rest machining), V-carving (medial axis via boostvoronoi, ADR 0015), centre lines of traced strokes, follow paths, holes (drill cycles, helical bores), surfacing, merging overlapping outlines (text), job planning |
| `chipload-post` | post-processor configs and templates (`posts/`, `templates/`) |
| `chipload-check` | the **independent** G-code checker |
| `chipload-pipeline` | app services: job + DXF → checked G-code, report text, the material-removal simulation (height map) |
| `chipload-cli` | the `chipload` command line (`--json` on every command through `out::Out`; no printing elsewhere) |
| `chipload-gui` | the egui desktop app (`chipload-gui`) |

## Architecture rules (non-negotiable)

Enforced on the resolved dependency graph by `crates/chipload-cli/tests/architecture.rs`.

1. `chipload-core` is **headless** and at the bottom: no I/O, no GUI crates, no other workspace crate.
2. Dependency direction: `gui/cli -> pipeline -> toolpath/post/dxf/job -> core`. Never the reverse; nothing depends on `chipload-gui` or `chipload-cli`.
3. `chipload-check` must never depend (even indirectly) on `chipload-toolpath`, `chipload-post`, `chipload-dxf` or `chipload-pipeline`: its value is that it can disagree with the generator.
4. Only `chipload-gui` may use GUI libraries (egui, eframe, winit, rfd).
5. Every feature must be usable from the CLI before it gets a UI.
6. The UI is thin: it builds a Job, calls the pipeline, displays results. No geometry or toolpath logic in `chipload-gui`.
7. Post-processors are **data-driven** (Jinja templates via minijinja + a small TOML config), never hardcoded per machine.

## Clean-room implementation (non-negotiable)

Chipload is a clean-room implementation in Rust, written from scratch in this repository
(user rule, 2026-10-10, made explicit 2026-10-17). Its value and its licence depend on every
line being its own.

- **Never copy, port, translate or paraphrase code** from another CAM program or toolpath
  library, open source or proprietary, whatever its licence (FreeCAD, PyCAM, OpenCAMLib,
  Kiri:Moto, CamBam, Fusion 360, Vectric, Estlcam, Carbide Create, and so on). Don't read
  their source to implement a feature here. GPL-compatible doesn't make it allowed.
- **Don't copy their other text either:** post-processor templates, help pages, UI wording,
  icons, sample files or project formats. Write templates from the controller's own public
  documentation (GRBL, LinuxCNC, Mach3/4 manuals) and the G-code standard (RS-274/NIST).
- **Build only on publicly available, general knowledge:** textbooks, published papers,
  standards, controller and tooling manufacturers' public documentation (feeds, chip loads,
  G-code dialects), well-known algorithms (offsetting, medial axis, Voronoi diagrams, marching
  squares), and first principles. Name the source in the spec when an algorithm comes from
  one.
- **Other programs may be named for what users expect**, at the level of a feature
  ("CamBam calls this Parts", "a Browser like Fusion 360's"), never for how they do it
  inside. No reverse engineering of binaries, file formats or output to reproduce them.
- **Dependencies are used, not copied:** a crate is used through its API under its own
  licence (the dependency rules below). Code is never lifted out of a dependency into this
  repository.
- **When in doubt about where an idea or snippet comes from, stop and ask.** Code suggested
  from memory that resembles a specific project's source is treated the same as copying it:
  write it again from the spec.

## Conventions

- **Units:** internal geometry and toolpaths are always **millimetres**. Convert only at the edges (DXF import via `$INSUNITS`, UI display, post-processor output). Never guess units silently: if the DXF has no units, require an explicit choice and record it in the job.
- **Coordinates:** right-handed, Y up, angles in **radians** internally, counter-clockwise positive.
- **Tolerances:** all in `chipload_core::tolerances` (chaining/joining 0.001 mm, arc approximation 0.01 mm). Never write a bare epsilon like `1e-6` in algorithm code. Use the named constant.
- **Floats:** `f64` everywhere.
- **Errors:** typed errors (`thiserror`) per crate. No `unwrap()`/`expect()`/`panic!` in library code (workspace lints deny them; tests may). `#![forbid(unsafe_code)]` via workspace lints.
- **Determinism:** same input + same job file must produce byte-identical G-code. No timestamps in output unless a template asks for them. No `HashMap`/`HashSet` iteration in anything that reaches output (use `BTreeMap`/`BTreeSet`). Explicit number formatting.
- **Style:** `cargo fmt`, `cargo clippy -D warnings`. Small functions, no clever one-liners in geometry code.
- New source files start with `// SPDX-License-Identifier: GPL-3.0-or-later`.

## Persisted formats and schema versioning

Anything saved to disk and read back later (job files, tool libraries, machine profiles, post-processor configs, app settings) is a **persisted format**. Users keep these files for years, so each one is versioned from the first commit.

- Every persisted file has a top-level integer `schema_version`. Also record `written_by` (e.g. `"chipload 1.4.2"`) to help debug customer files.
- Each format has a `CURRENT_VERSION` constant and one migration function per step that upgrades version N data to N+1 (jobs: `chipload_job::migrations`; post configs: `chipload-post`).
- Loading always goes through one function per format (e.g. `load_job`). It must:
  - **Older version:** apply migrations step by step to the current version, then validate.
  - **Current version:** validate and load.
  - **Missing, malformed or newer-than-known version:** return a typed error (e.g. `JobError::UnsupportedVersion`) and refuse to load. Never guess.
- Validation code only ever targets the latest format. Old formats are handled solely by migrations.
- **Bump the version** whenever a change would make older code misread a file: rename, removal, new required field, or changed meaning/units of a value. Adding an optional field with a safe default does not need a bump.
- **Every schema change ships in the same commit as:**
  1. the version bump,
  2. the migration function,
  3. a frozen sample file of the old version in `tests/corpus/jobs/v<N>/`,
  4. a test that the sample migrates and still produces the expected golden G-code.
- Never edit or delete a frozen old-version sample. Add a new one instead.
- Exception, decided 2026-10-10: formats from before the first public release may be dropped
  as a whole when the user decides so. Schema 1-3 (jobs before projects) were dropped: the
  oldest version that loads is `OLDEST_VERSION` (4), older files are refused with advice.
  From the first public release on, every version stays loadable.
- Migrations must be pure functions (no I/O, no clock) and must not silently change machining behaviour. If a migration cannot preserve behaviour exactly, stop and ask.
- Saving always writes the current version, with every field written explicitly (`save_job`). Never write an older format.
- Treat missing units, sides, directions, or depths in an old file as a migration decision to surface, not something to default quietly.

## Commands

```
cargo test --workspace                                    # all tests
cargo test -p chipload-toolpath                           # one crate, fast loop
cargo test -p chipload-cli --test golden                  # G-code regression
CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p chipload-cli --test golden   # ONLY when a change is intended; review the diff
cargo test -p chipload-pipeline --test sheet               # golden setup sheets (same CHIPLOAD_UPDATE_GOLDEN rule)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check licenses bans sources                    # GPL-3.0 compatibility of every crate
cargo check --target wasm32-unknown-unknown -p chipload-core -p chipload-job -p chipload-dxf -p chipload-image -p chipload-text -p chipload-toolpath -p chipload-post -p chipload-check -p chipload-pipeline   # the libraries still compile for the browser (docs/specs/web.md)

cargo run -p chipload-cli -- import part.dxf -o part.chipload       # new project from a DXF
cargo run -p chipload-cli -- new -o part.chipload                   # empty project (DXF later via update)
cargo run -p chipload-cli -- copies part.chipload --grid 3x2 --gap 10   # a grid of copies of the part (--part ID with several)
cargo run -p chipload-cli -- part add part.chipload --name box   # parts: groups of operations cut one after the other (also part list/set/activate/remove, op part)
cargo run -p chipload-cli -- place part.chipload --align bottom-left --margin 10   # move the imported drawing onto the stock
cargo run -p chipload-cli -- place part.chipload --rotate 30  # turn it in place, any angle (also --at X,Y: the box's corner)
cargo run -p chipload-cli -- place part.chipload --width 100   # scale it about its centre (also --scale 2, --height)
cargo run -p chipload-cli -- inspect part.chipload        # (or part.dxf) units, contours, issues
cargo run -p chipload-cli -- run part.chipload -o out.nc  # headless end to end (several tools: out-1-T1.nc, … or --tool-change program); also out-sheet.html (--no-sheet)
cargo run -p chipload-cli -- sheet part.chipload -o sheet.html   # the operator's setup sheet alone (HTML)
cargo run -p chipload-cli -- op tool part.chipload --op 2 --tool 3   # operation 2 cuts with project tool 3
cargo run -p chipload-cli -- op rest part.chipload --op 3 --after-tool 1   # pocket 3 clears only what tool 1 leaves (--none: whole pocket)
cargo run -p chipload-cli -- op add-rest part.chipload --op 1 --tool 2   # a rest pocket after pocket 1: same shapes and depths, tool 2
cargo run -p chipload-cli -- op add-surface part.chipload --depth 0.5   # face the whole stock (spoilboard), first in the active part
cargo run -p chipload-cli -- plan part.chipload --json    # toolpath, time estimate and chip load, no G-code
cargo run -p chipload-cli -- simulate part.chipload -o sim.png   # material-removal simulation: numbers and a shaded top view
cargo run -p chipload-cli -- shape mirror part.chipload 2 3 --axis x   # also shape add/set/delete, shape copy IDS --dx --dy
cargo run -p chipload-cli -- shape set part.chipload 1 --corner-radius 5   # round a rectangle's corners (at most half its shorter side)
cargo run -p chipload-cli -- update part.chipload --from part.dxf  # new version of the drawing (--dxf ID with several)
cargo run -p chipload-cli -- dxf add part.chipload lid.dxf --name lid   # another DXF in the project (also dxf list/remove; place --dxf ID)
cargo run -p chipload-cli -- image add part.chipload logo.png --width 120   # trace a picture into shapes on layer "logo" (also image set/list/preview/remove; --trace centre-line, --arcs false)
cargo run -p chipload-cli -- text add part.chipload "MAKER\nSPACE" --height 20 --at 40,30   # text in a built-in font (also text set/list/remove/fonts, place --text)
cargo run -p chipload-cli -- set part.chipload machine.gcode_units=inch   # G-code in inches (G20); the project stays mm (docs/specs/inch-units.md)
cargo run -p chipload-cli -- text add part.chipload "SIGN" --height 1/2in --at 1in,2in   # any length option takes a unit; a plain number is mm
cargo run -p chipload-cli -- check out.nc --job part.chipload      # the independent checker
cargo run -p chipload-cli -- get part.chipload operations.0.end_depth   # any value by path (0-based, as problem paths)
cargo run -p chipload-cli -- set part.chipload operations.0.end_depth=6.5  # checked, then written; `run/plan --set PATH=VALUE` for one run
cargo run -p chipload-cli -- check-project part.chipload           # the project's problems with their paths
cargo run -p chipload-cli -- schema project                        # JSON Schema (also tools, machines); copies in docs/schemas/
cargo run -p chipload-cli -- plan part.chipload --json             # --json on any command: one object (docs/book/src/scripting.md)
cargo run -p chipload-cli -- mcp                                   # MCP server on stdio: every command as a tool for AI assistants (docs/specs/mcp.md)
cargo run -p chipload-cli -- tool list                   # the user's tool library (also: machine list)
cargo run -p chipload-cli -- tool compare part.chipload  # project tools vs their library entries, by uid (also: machine compare)
cargo run -p chipload-cli -- tool add "V 60" --type vbit --diameter 12.7 --angle 60 --tip 0.2   # a V-bit for V-carving
cargo run -p chipload-cli -- tool export tools-backup.json   # (also machine export); import with `tool import FILE [--replace]`
cargo run -p chipload-cli -- post list                   # built-in post-processors (grbl, linuxcnc, mach3) and yours (settings folder posts/); post show NAME
cargo run -p chipload-cli -- post use part.chipload --post linuxcnc   # a user post is copied into the project
cargo run -p chipload-gui -- part.chipload                # GUI (also opens a .dxf)
cargo about generate --fail about.hbs > THIRD-PARTY-LICENSES.txt   # after dependency changes (CI checks it)
packaging/linux/build-appimage.sh 0.2.0 dist              # Linux AppImage (after cargo build --release)
```

Releases: push a `vX.Y.Z` tag; CI builds the AppImage, Windows executables and a **draft**
release (`docs/releasing.md`, ADR 0010). Never push tags or publish without being asked.

Run the relevant tests after every change. Do not report a task done until they pass.

## Workflow expectations

- Work in **small steps**, one feature or fix per commit, with a clear message.
- For any new toolpath operation or non-trivial algorithm: write a short spec in `docs/specs/<name>.md` first (inputs, outputs, edge cases, what "correct" means), get it reviewed, then write tests, then implement.
- Prefer tests first for geometry. Add a `proptest` property test where an invariant exists (e.g. offset contour never crosses the original, pocket stays inside boundary).
- **Verify APIs against the version in `Cargo.lock`** (read the source in `~/.cargo/registry/src/`), don't rely on memory. This matters for `dxf`, `cavalier_contours`, `minijinja` and egui/eframe, whose APIs change between versions.
- If something is ambiguous (units, offset side, climb vs conventional, machine behaviour), **stop and ask** instead of choosing silently.
- Do not add dependencies without stating the license and why it's needed. Every dependency must be compatible with **GPL-3.0-or-later**. Allowed: MIT, BSD-2/3-Clause, ISC, Zlib, Apache-2.0, MPL-2.0, LGPL-2.1-or-later/3.0, GPL-2.0-or-later, GPL-3.0(-or-later), Unicode-3.0, BSL-1.0 (approved 2026-10-09). **Forbidden:** GPL-2.0-only (incompatible), proprietary, unknown or no license. **Needs explicit approval:** AGPL-3.0 (adds network obligations) and anything not listed. `deny.toml` encodes this list.
- Keep `docs/architecture.md` (the overview for contributors) right when a crate, a layer,
  a dependency between crates or the steps of `run_job` change.
- Keep `docs/decisions/` updated with a short ADR for any significant architectural or library decision.
- **The user guide** (`docs/book/`, mdBook, docs/specs/user-guide.md) is kept up to date in the
  same commit as the feature: a commit that adds or changes anything a user sees or uses
  updates its guide page. The reference pages (`docs/book/src/reference/`) are generated
  (`chipload reference …`) and checked by `cargo test -p chipload-cli --test guide`;
  regenerate them with `CHIPLOAD_UPDATE_GOLDEN=1`. Build the site with
  `packaging/docs/build-book.sh` (mdBook 0.5.4); renew its screenshots with
  `packaging/docs/screenshots.sh` (the GUI's `screenshots` feature).

## Safety rules for toolpath and G-code code

These bugs break tools, stock, and machines. Treat them as high-risk:

- Offset side (inside/outside), climb vs conventional, and spindle direction must be explicit in the job model and covered by tests.
- Never emit a rapid (G0) move below the safe height while stock lies in the path. Rapid-through-material is checked by `chipload-check`.
- Every generated program must pass the independent checker in `chipload-check` (stays inside machine limits, arc endpoints consistent with centre/radius, no feed of zero, no unit mismatch, retract before XY travel). `run_job` refuses to return G-code that fails it.
- Every offset is verified after it is built (distance to the contour, correct side, no gouge). A library result is never trusted blindly; a panic inside a geometry library becomes an error.
- Arc output (G2/G3) must be validated: start/end radius mismatch beyond tolerance is an error.
- Changing post-processor output or any golden file requires an explicit note in the commit message explaining why.
- Tabs, lead-ins, ramps, and multi-pass depths must be tested at boundaries (zero, equal to stock thickness, tool larger than feature).

## DXF handling notes

- Read with the `dxf` crate (ADR 0005). Treat all input as hostile and messy: duplicate entities, gaps, overlaps, zero-length segments, tiny arcs, splines, ellipses, bulged polylines, blocks/inserts with transforms, mixed layers, 3D-coordinates with nonzero Z. The parser runs under `catch_unwind`, and local-time dates are stripped before parsing (a `dxf` 0.6.2 bug).
- Pipeline: read -> expand blocks/transforms -> convert to Line/Arc primitives (approximate splines and ellipses within tolerance) -> remove duplicates/zero-length -> chain into contours with tolerance -> classify closed/open -> nesting (outer vs holes) -> report issues.
- Read the drawing as drawn: as little repair as possible, and never silently drop geometry. Anything ignored or repaired is reported in the `ImportReport` that the CLI and UI both show.
- DWG is not supported. Converting through the ODA File Converter (proprietary, never bundled) or LibreDWG (GPL-3.0-or-later) would be a separate decision with its own ADR.

## Test corpus

- `tests/corpus/dxf/` holds real and synthetic DXFs; `tests/corpus/expected/` holds golden G-code (with arcs); `expected/lines/` the same programs without arcs (post `arcs = false`), frozen when arc output came (ADR 0013); `expected/sheets/` golden setup sheets.
- `tests/corpus/jobs/v<N>/` holds frozen projects for every schema version still loaded (v4 on; see schema versioning above). The golden test runs them through their migrations and must give the same G-code.
- When a user-reported file breaks something, add it (anonymised) to the corpus **with a failing test first**, then fix.

## Licensing and packaging

- The project is GPL-3.0-or-later (`LICENSE`). Contributions are accepted under the same license with a DCO sign-off (`CONTRIBUTING.md`).
- Packaging scripts live in `packaging/`. Don't hardcode secrets or certificate paths. Read them from environment variables.
- Every binary carries `LICENSE` and `THIRD-PARTY-LICENSES.txt` (`chipload_pipeline::notices`). Regenerate the notices with cargo-about whenever dependencies change.

## Things to avoid

- No global mutable state.
- No printing from library crates (`print_stdout` is denied); report through return values.
- No catch-all error handling that hides geometry errors. Return typed errors.
- No network calls from library crates.
- Don't "fix" a failing golden test by regenerating it without understanding the diff.
- Don't change a persisted format without a version bump and migration. Don't add a new persisted format without `schema_version`.
