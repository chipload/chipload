# Spec: user guide (documentation website)

Status: **approved 2026-10-13** ("All yes"), all decisions as recommended (§8).

User request (2026-10-13): a documentation website that is always kept up to date with the
app's features. Decided already:
- it lives in this repository;
- it is built locally, with no CI;
- the repository stays private for now and becomes public later, and the site is
  published then.

## 1. What it is

A user guide for people who cut parts with Chipload: what each feature does, how to use it
in the GUI and on the command line, and what to watch out for on the machine.

Not in it:
- the specs (`docs/specs/`) and ADRs (`docs/decisions/`). They stay the developers'
  documents, describing how and why;
- the guide says what and how to use it, and links to a spec only for exact behaviour.

## 2. Tool: mdBook

- **What:** Markdown in, a static HTML site out, with search, a sidebar and light and dark
  themes. MPL-2.0. The Rust ecosystem's usual choice.
- **Not a dependency:** it is a build tool and is never linked into Chipload, so it isn't in
  `Cargo.lock` or the third-party notices.
- **Pinned:** the build script checks for mdBook **0.5.4** and says how to install it if
  it's missing (`cargo install mdbook --version 0.5.4 --locked`).
- **Where:** sources in `docs/book/` (`book.toml`, `src/SUMMARY.md`, one Markdown file per
  page). The built site goes to `target/book/`, never committed.

## 3. Contents

Written for an operator who knows CNC but not Chipload:

1. **Getting started:** install (AppImage, Windows `.exe`), the first part from a DXF to
   G-code, and checking the program.
2. **Concepts:**
   - units;
   - stock, work zero and Z zero;
   - depths, clearance and the stock bottom;
   - inside, outside and on the line; climb and conventional;
   - tool groups and tool changes;
   - the independent checker and what it guarantees.
3. **Drawings:**
   - DXF import and its report;
   - placing, turning and scaling;
   - updating from a new DXF;
   - drawing circles, rectangles and polylines;
   - a grid of copies.
4. **Operations**, one page each:
   - profile: tabs, leads, spiral laps, finishing pass, onion skin, dogbones;
   - pocket: rings, helix, start points, rest machining;
   - V-carve;
   - follow path;
   - holes.
5. **Tools and machines:** the library, cutting values, machine profiles, import and
   export.
6. **Post-processors:** the built-in ones, writing your own, and testing it.
7. **Before cutting:** the problems list, the 3D view, the simulation, playback, cycle time,
   chip load, and the setup sheet.
8. **Scripting:** today's `docs/scripting.md`, moved into the book.
9. **Reference, generated (§4):** command line, project file, post-processors.
10. **Troubleshooting:** common import issues and refused programs, with what each message
    means.

Pictures: a few GUI screenshots where words don't do it (the canvas, the playback slider).
`packaging/docs/screenshots.sh` makes them, using the GUI's existing screenshot feature
(decision 3).

## 4. Kept up to date

**Generated pages, checked by tests.** These are the pages most likely to go stale, so they
are made from the code itself:
- **Command line** (`reference/cli.md`): every command and option, with its help text, from
  the CLI's clap definitions.
- **Project file** (`reference/project-file.md`): every field with its type, whether it is
  required, its allowed values and its description, from the JSON Schema (scripting.md §2).
  Tool and machine files the same way.
- **Post-processors** (`reference/posts.md`): the built-in posts with their settings.

A hidden command, `chipload reference cli|project|tools|machines|posts`, prints each page.
A golden-style test compares each with its committed page, and `CHIPLOAD_UPDATE_GOLDEN=1`
rewrites them, so a new option or field shows up as a failing test until the page is
regenerated.

**Written pages, by rule.**
- A new rule in CLAUDE.md: a commit that adds or changes a user-facing feature updates the
  guide's page for it in the same commit, as TODO.md is kept today.
- Each spec gains a line naming its guide page.

**A cross-check test:**
- every operation type, every CLI command, and every option of the project file's
  operations is mentioned on some written page, not only in the reference;
- the test lists what is missing;
- the check is a plain word search, so it can't judge quality, but nothing new goes
  unmentioned (decision 4).

**Links:**
- the build fails on broken internal links (mdBook's own check);
- links into `docs/specs/` work while the repository is the source, and become links to
  the repository on GitHub once it's public.

## 5. Building, now and later

- **Now:** `packaging/docs/build-book.sh`:
  - checks the mdBook version;
  - checks that the generated pages are current (the test above);
  - builds to `target/book/`;
  - prints the path to open in a browser;
  - the site works opened from disk (`file://`), search included.
- **Version:** the guide shows the Chipload version it was built for, taken from
  `Cargo.toml`, on every page.
- **Releases:** `docs/releasing.md` gains a step to build the guide and attach it to the
  release as `chipload-guide-<version>.zip`. Testers of the private repository get it that
  way.
- **Later, when the repository is public:**
  - `packaging/docs/publish-book.sh` builds the guide from a release tag and pushes it to a
    `gh-pages` branch, for GitHub Pages;
  - it is written now but not run until then.
  - Only released versions are published, so the site never shows features that aren't out
    yet.

## 6. In the app

The GUI's Help menu gains **User guide**:
- it opens the published guide once there is one;
- until then it opens the guide shipped with the app, if packaged (decision 5), else a
  message saying where to find it.

## 7. Tests

- Each generated page equals what `chipload reference …` prints (golden-style).
- Coverage: every operation type, command and operation field is mentioned on a written
  page.
- `build-book.sh` runs in the release checklist, not in `cargo test`, since mdBook isn't a
  Rust dependency. The two tests above are plain Rust tests and run always.

## 8. Decisions

1. **mdBook**, sources in `docs/book/`. **Recommended**: simple, Markdown, the Rust
   ecosystem's standard. Alternatives: Zola or MkDocs (Python), more themable but another
   toolchain.
2. **The command-line and project-file references are generated and checked by tests.**
   **Recommended**: they can't drift. Alternative: written by hand, which goes stale.
3. **A few screenshots, made by a script before each release.** **Recommended**: words
   first, pictures where they really help. Alternative: none, which makes the canvas pages
   harder to follow.
4. **A coverage test by word search** (every operation, command and field is mentioned).
   **Recommended**: cheap, and catches a forgotten page. Alternative: the CLAUDE.md rule
   alone.
5. **Bundle the built guide with the AppImage and Windows downloads?** **Recommended: no.**
   Releases carry it as a separate zip, and the Help menu links to the website once it's
   public. Bundling adds a few MB to every download and a second copy to keep right.
   Alternative: bundle it, so it opens offline.
6. **Move `docs/scripting.md` into the book** (its first page), with a stub left at the old
   path pointing there. **Recommended**: one guide.
7. **Write the first version of every page now**, not only the generated ones.
   **Recommended**: the guide is useful from day one. It is a lot of writing, so it comes
   in several commits, one per section. Alternative: start with the reference and getting
   started, and fill in the rest as features change.
