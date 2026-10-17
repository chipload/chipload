# Post-processors

A post-processor turns Chipload's toolpath into the G-code dialect your controller reads.
Three are built in:

| Post | For | Files | Tool changes |
|---|---|---|---|
| `grbl` | GRBL 1.1 routers (and senders such as UGS, CNCjs, gSender) | `.nc` | one file per tool, or one file with `T… M6` for a sender that handles the change |
| `linuxcnc` | LinuxCNC | `.ngc` | one file per tool |
| `mach3` | Mach3 and Mach4 | `.tap` | one file per tool |

**The LinuxCNC and Mach3/4 posts haven't been run on a real controller yet.** Their
programs say so in a header comment: check the program and air-cut before you cut. If you
run one of them successfully, please report it.

Every post writes arcs as G2/G3, mm and absolute coordinates, and waits 3 s after starting
the spindle. The [post-processor reference](reference/posts.md) lists every post's
settings.

## Choosing the post

- **App:** Job settings > Machine > Post. The list shows the built-in posts, then yours.
- **Command line:** `chipload post use part.chipload --post linuxcnc`.
- **Machine profile:** a profile names its post, and a project made from the profile gets
  it.

## Your own post

A post is two files in the `posts` folder of your [settings folder](install.md#your-settings):

- `NAME.toml`, the settings ([reference](reference/posts.md));
- the template the settings name (`template = "NAME.nc.j2"`), a
  [Jinja](https://docs.rs/minijinja/latest/minijinja/syntax/index.html) text that writes the
  program.

The easiest start is a copy of a built-in post, from the `crates/chipload-post/posts/` and
`templates/` folders of the source. Rules:
- **Names:** a post can't take a built-in name (`grbl`, `linuxcnc`, `mach3`).
- **Copied into the project:** a project that uses your post keeps its own copy of both
  files, so it gives the same G-code on another computer or after you change the files.
  When your files differ from the project's copy, the app shows **Update** next to the Post
  box (command line: run `post use` again).
- **Sandboxed:** the template sees only the values below, can't read files, and stops with
  an error if it loops without end.
- **The checker still decides.** Whatever your template writes, the [checker](first-part.md#the-checker)
  reads it back as a controller would. It accepts only the codes it knows: G0–G3, G4, G17,
  G20, G21, G90, G91.1, G94, the setup codes G40, G49, G80, G54, G61 and G64 P (at most
  0.05 mm), M3–M6, M7–M9 and M30, and `%` lines. Anything else, such as G41/G42 cutter
  compensation, G43 tool lengths, G28 or G53 machine moves, or G91 relative moves, makes the
  check fail, and nothing is exported.

`chipload post list` shows every post and why one can't be used, and `chipload post show
NAME` what one writes (extension, decimals, arcs, tool changes). In the app they are under
*This computer > Post-processors* in the Browser. `chipload render` writes a
bare toolpath through a post, for trying a template (the result isn't checked: use `run` to
cut).

### What the template sees

| Value | |
|---|---|
| `post_name` | the settings' `name` |
| `units_word` | `G21` or `G20`: the machine's *G-code units* |
| `unit` | `mm` or `inch`, for comments |
| `tool_id`, `tool_diameter`, `rpm` | the (first) tool, its diameter in the program's units, its spindle speed |
| `spindle` | `"cw"` or `"ccw"` |
| `safe_z` | the safe height, in the program's units |
| `dwell` | seconds to wait after starting the spindle, or none |
| `work_zero` | where the work zero is, as text for a comment, or none |
| `file` | with one file per tool: `number`, `count`, `tool` (text) and `operations` (text); otherwise none |
| `tools` | with one file and tool changes: a list of the tools as text; otherwise none |
| `segments` | the program in parts: one, or one per tool with tool changes |
| `segment.change` | before every part but the first: `comment`, `tool_id`, `rpm`; otherwise none |
| `segment.moves` | the moves: `code` (`G0`, `G1`, `G2`, `G3`), and `x`, `y`, `z`, `i`, `j`, `f` (each none when unchanged) |
| `move.comment` | text to write as a comment before the move, made safe for G-code: where each [part](operations/parts.md) starts, with two parts or more; otherwise none |

Numbers come already formatted in the program's units: with the settings' `decimals` in mm,
`inch_decimals` (4 if missing) in inches.

**Inches.** A post writes inch G-code only if its settings say its template can:
`units = ["mm", "inch"]`, and the template writes `{{ units_word }}` where it would write
`G21`. A post without `units` writes mm only, and a project whose machine is set to inch is
refused with that post, so a template that writes `G21` itself never gets inch numbers.

The built-in GRBL template shows how they fit together:

```jinja
{{ units_word }} G90 G17 G94
{{ "M3" if spindle == "cw" else "M4" }} S{{ rpm }}
{% if dwell is not none %}G4 P{{ dwell }}{% endif %}
{% for s in segments %}
{% for m in s.moves %}
{% if m.comment is not none %}
({{ m.comment }})
{% endif %}
{{ m.code }}{% if m.x is not none %} X{{ m.x }}{% endif %}{% if m.y is not none %} Y{{ m.y }}{% endif %}{% if m.z is not none %} Z{{ m.z }}{% endif %}{% if m.i is not none %} I{{ m.i }} J{{ m.j }}{% endif %}{% if m.f is not none %} F{{ m.f }}{% endif %}
{% endfor %}
{% endfor %}
M5
M30
```
