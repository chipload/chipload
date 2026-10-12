# 0016: User post-processors, copied into projects; setup codes in the checker

Status: accepted (2026-10-12)

Users can add their own post-processors: a post config (schema 1, ADR 0003/0007) and the
template it names, in `posts/` in the settings folder. Built-in posts are now `grbl`,
`linuxcnc` and `mach3` (docs/specs/user-posts.md).

## Copied into the project
When a project chooses a user post, the config's and template's text are copied into the
project (`machine.custom_post`, project schema 15), as tools and machines are (ADR 0014).
The project's G-code then depends only on the project and the Chipload version, on any
computer, also after the file in `posts/` changes; the GUI offers Update when it has, the CLI
`chipload post use` again. Built-in posts stay referenced by name: they change only with
Chipload and the golden tests cover them.

## The checker stays the judge
A user template can write anything, so the independent checker, not the template, decides
what reaches a machine. It now also accepts a few codes that only put a LinuxCNC or Mach
controller in a known state: `%` lines, G40, G49, G80, G54, G61, G64 with P at most 0.05 mm,
G91.1 and M7/M8/M9. Codes that would move the tool off the checked path stay refused
(G41/G42, G43, G28/G30/G53, G55–G59, G90.1, G91).

## Sandboxing
Templates render in minijinja with no loader (they see only the context), strict undefined,
and a step limit (the crate's `fuel` feature, no new dependency), so a template that loops
without end fails instead of hanging.
