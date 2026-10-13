# Spec: your own post-processors, and LinuxCNC and Mach3/4 posts

Status: approved (2026-10-12; decisions 1–8 as recommended). Item 5 of the general improvement pass (TODO.md, approved
2026-10-12).

Guide: docs/book/src/posts.md.

## 1. Why

Chipload ships one post-processor, GRBL, and the pipeline only loads built-in posts,
although posts are data (a TOML config and a Jinja template, ADR 0003/0007) so that users
can write their own. Anyone with LinuxCNC, Mach3/4 or a controller with its own dialect
can't use Chipload today.

## 2. Your own posts

- **Where:** `posts/` in the settings folder (next to `tools.json`). A post is
  `<name>.toml`, a post config as today (schema 1), and the template file its `template`
  names, in the same folder. The machine's `post` is that `<name>`.
- **Same rules as built-ins:** the config is validated as now; the template renders in the
  same sandbox (it sees only the context Chipload gives it, no files, strict undefined).
  Rendering gets a step limit (minijinja's `fuel`, a feature of the crate already used), so
  a template that loops without end fails with an error instead of hanging.
- **Names:** a user post can't take a built-in's name (`grbl`, `linuxcnc`, `mach3`); such a
  file is listed with a note and not used.
- **Copied into the project** when chosen, like tools and machines (ADR 0014): the project
  keeps the config text and template text it was set up with, so it gives the same G-code
  on another computer, or after the file in `posts/` changes. When the file in `posts/`
  differs from the project's copy, the GUI says so next to the Post box with **Update**
  (CLI: `chipload post use`). Built-in posts stay by name: they change only with Chipload,
  and the golden tests cover them.
- **The checker still decides.** Whatever the template writes, the independent checker
  reads the program as a controller would and refuses anything it doesn't know (§4). A post
  that writes something outside that list fails the check, and nothing can be exported.

## 3. Project schema 15

- `machine.custom_post`: null for a built-in post, or `{ "config": "<TOML text>",
  "template": "<template text>" }`, the user post as copied. When set, its config's name
  must match `machine.post`.
- Migration `v14_to_v15` adds `custom_post: null`: same post, same G-code (the frozen v14
  sample `v14/round_corners` and its golden cover it). The bump makes an older Chipload refuse
  a v15 file as newer instead of failing on the unknown field.

## 4. The checker learns a few setup codes

LinuxCNC and Mach programs usually start by putting the controller in a known state.
The checker accepts these, and only these, in addition to today's codes:

| Code | Meaning | Rule |
|---|---|---|
| `%` alone on a line | program start/end marker | ignored |
| G40 | cutter radius compensation off | — |
| G49 | tool length offset off | — |
| G80 | canned cycle off | — |
| G54 | the first work offset (the default) | G55–G59 stay refused: they move the program |
| G61 | exact stop | — |
| G64 P… | path blending within P mm | P required, at most 0.05 mm |
| G91.1 | arc centres I/J relative to the start (what Chipload writes) | G90.1 (absolute I/J) refused |
| M7, M8, M9 | coolant on/off | — |

Everything else stays refused, among them G41/G42 (compensation would move the tool off the
checked path), G43 (tool length from a table the checker can't see), G28/G30/G53 (moves to
machine positions, possibly through the stock) and G91 (relative moves).

## 5. Built-in LinuxCNC and Mach3/4 posts

Written for Chipload (no other post's code is copied), from the controllers' documented
G-code.

- **linuxcnc:** `%`, `G21 G90 G17 G94 G40 G49 G80 G54 G64 P0.01 G91.1`, spindle, a dwell
  `G4 P<s>` (LinuxCNC's P is seconds), arcs as G2/G3, `M5 M30`, `%`.
- **mach3** (also for Mach4): `%`, `G21 G90 G17 G94 G40 G49 G80 G54 G91.1`, spindle, `G4 P<s>`,
  arcs as G2/G3, `M5 M30`, `%`. A header comment notes that Mach3 can be set to read G4 P in
  milliseconds; if so, the dwell is shorter than intended and Mach's own spin-up delay
  should be set.
- **One file per tool only** for both (`tool_changes = ["files"]`), as each file asks for Z
  to be set with its tool. One file with M6 needs tool length offsets (G43 H) from the
  controller's tool table, which Chipload can't check; that is a later decision.
- Neither has been run on a real controller yet: their header comment says so, and
  the docs ask for an air cut first. The note comes off once a user confirms a post.

## 6. CLI and GUI

- `chipload post list`: built-in and user posts, with a note for each that can't be used
  and why.
- `chipload post use part.chipload --post NAME`: set the project's post; a user post is
  copied in (again: updates the copy).
- GUI: the Machine section's Post box lists built-in posts, then yours. Picking one of yours
  copies it in; "Update" appears when your file has changed since. *Settings → Posts
  folder* shows where to put them.
- Machine profiles keep naming posts by name (`machines.json` unchanged); choosing a profile
  whose post is one of yours copies it in.

## 7. Decisions to review

1. **User posts in `posts/` in the settings folder**, config and template side by side.
   *Recommended.*
2. **Copied into the project when chosen** (schema 15), with Update when the file changes;
   built-ins by name. *Recommended:* the same as tools and machines (ADR 0014), and a
   project's G-code doesn't depend on a file outside it.
3. **User posts can't reuse a built-in name.** *Recommended:* no doubt which one cuts.
4. **The checker accepts the setup codes in §4**, G64 only with P ≤ 0.05 mm, and nothing
   else new. *Recommended:* enough for LinuxCNC and Mach headers; nothing that moves the tool
   off the checked path.
5. **LinuxCNC: G64 P0.01 and G91.1.** *Recommended:* blending within 0.01 mm (the arc
   tolerance) keeps motion smooth without cutting corners visibly; G91.1 makes the I/J
   meaning explicit.
6. **Mach3/4: G91.1, G4 P in seconds, no G64.** *Recommended:* Mach's IJ mode is a setting
   that can be absolute; G91.1 overrides it. The dwell in seconds is never worse than no
   dwell (if Mach reads milliseconds it is just short).
7. **Files per tool only** for LinuxCNC and Mach, no G43. *Recommended:* one file with tool
   changes is a separate decision about tool length offsets.
8. **"Not yet tested on a real controller" in their header** until a user confirms.
   *Recommended.*

## 8. Tests

- Post crate: a user post loads from a folder; a missing template, an unsafe template name,
  a built-in name and an endless template loop give clear errors.
- Checker: each new code accepted as listed; G64 without P or with P > 0.05, G90.1, G55,
  G43, G41, G28 refused; `%` lines ignored.
- Golden files for `linuxcnc` and `mach3` (an existing corpus project with arcs, a helix
  and tabs, with the post changed), each passing the checker; the lines-only versions too.
- Project schema 15: the v14 sample migrates with `custom_post: null` and gives the same
  G-code; a project with a copied post renders from its copy after the file in `posts/` is
  deleted or changed.
- CLI `post list` and `post use`; GUI: picking a user post copies it, Update after the
  file changes.

## 9. As built (2026-10-12)

- `chipload_post`: `builtin_posts`, `read_user_post`, `list_user_posts`, `parse_user_post`,
  `UserPost`; `PostConfig::template_text` (not in the file) holds a user template. Render
  fuel: 1 000 000 steps plus 2 000 per move.
- `machine.custom_post` (schema 15, `v14_to_v15` sets null). It is optional with a null
  default; the rule "its config's name must match `machine.post`" (§3) was dropped: `post`
  is the file name the copy came from, while a config's `name` is the display name (the
  built-in `grbl` is called "GRBL").
- `chipload_pipeline::posts`: `job_post` (what `run_job` renders with), `post_choices`,
  `choose_post`, `copy_state`, `post_exists`; `library::copy_profile_post` copies a
  profile's user post when the profile is applied (new projects, `machine use`, GUI). `chipload
  machine add/set --post` accepts a user post that can be read.
- CLI `post list`, `post use`. `chipload render` (a bare toolpath) still takes built-in
  posts only.
- GUI: the Post box lists built-in posts, then yours (unusable ones greyed, with why); a
  row under it says whether the project's copy matches your file, with Update.
- Golden: `v15_linuxcnc_helix`, `v15_mach3_helix` (v7_helix with the new posts: the same
  moves as `v7_helix`) and their lines-only versions; the lines-only test now renders each
  project with its own post.
