# Spec: rest passes under their pocket

Status: approved 2026-10-17 with the recommended decisions, built the same day. Builds on
docs/specs/rest-machining.md; its §10 (*Add rest pocket*, a copy) is replaced by this.

## 1. Why

Today a rest pocket is an operation of its own. *Add rest pocket ▸* (rest-machining.md §10)
copies the pocket's shapes and depths into it once; after that the two drift apart when the
pocket changes, and the warning in §5 is the only guard. In the Browser the rest pocket is
one more row, with nothing showing which pocket it finishes.

The user's idea: right-click a pocket, *Add rest machining*, and the rest becomes a leaf
under that pocket. This is option A of rest-machining.md §10, the live link we left for
later.

## 2. What it is

A **rest pass** belongs to a pocket. It clears what the pocket's tool (or the pass before
it) can't reach, with a smaller endmill.

- **Taken from its pocket, always:** the shapes, start and end depth, clearance, direction,
  and whether the pocket is cut (off, or its part off, means its passes aren't either).
- **Its own:** the tool, feed, plunge feed, step down, stepover, entry, and on/off.
- **Not:** start points and leads (they belong to the pocket's rings), dogbones (as now,
  rest-machining.md decision 3).
- **Several passes:** a pocket can have more than one, each with a smaller tool than the
  one before, each clearing what the one before it left (12 mm, then 6, then 3). Each pass
  rests after the previous pass's tool, or the pocket's for the first.

The rest region is computed exactly as today (rest-machining.md §2): from the larger
tool's reach, with this pass's clearance. Only where the values come from changes.

## 3. When it is cut (decision 2)

**After the run of pockets it belongs to:** the pocket and the pockets right after it in
the part that cut with the same tool. Then the rest passes of that run, smallest tool last,
grouped by tool. Then the operations after the run.

| Operations | Cut order | Tool changes |
|---|---|---|
| Pocket A (T1, rest T2), Pocket B (T1, rest T2), Outline (T1) | A, B, A's rest, B's rest, Outline | T1, T2, T1: 2 |
| right after each pocket instead | A, A's rest, B, B's rest, Outline | 4 |

So the small tool comes in once for all the pockets of a run, and the outline, which frees
the part, stays last. The order is shown: the Browser numbers each pass where it is cut, the
setup sheet lists them in cut order, and the G-code comment names the pocket
("Operation 2.1: rest of Mortise, T2").

## 4. The file

A pocket gets an optional `rest_passes` list; empty is left out when saved, as
`drawing.texts` is, so no schema bump: older Chipload refuses a file that has one, and reads
every other file as before.

```json
"rest_passes": [
  { "tool": 2, "feed": 900, "plunge_feed": 300, "step_down": 1.5, "stepover": 0.4,
    "entry": {"type": "helix", "radius": 1.0, "ramp_angle": 0.05}, "enabled": true }
]
```

Paths for `get`/`set` and problems: `operations.1.rest_passes.0.feed`.

## 5. The standalone rest pocket stays (decision 5)

A pocket's own `rest: {after_tool}` stays: "rest after T1 for these shapes", for a large
pocket cut elsewhere, in another job or program (rest-machining.md decision 4). It is set
under *Rest after* in the pocket's form, as now. It is the exception; a rest of a pocket in
this project is a rest pass.

Existing rest pockets are **not migrated**: they keep their place in the list and cut as
before, so no G-code changes. A rest pocket that copies an earlier pocket (same shapes,
depths, clearance, direction, and `after_tool` its tool) gets a one-click offer in its
right-click menu: *Make it a rest pass of operation N*. That moves it under N, where the
order rule of §3 applies; the report says when the cut order changed.

## 6. In the app and on the command line

- **Browser:** the pass is a leaf under its pocket, with its tool and time: "↳ Rest · T2 ·
  3 mm · 1 min 10 s". Right-click a pocket: *Add rest pass ▸* lists the endmills smaller
  than the last tool of its chain: the project's first, then the user's library's that
  aren't in the project yet. Picking a library tool copies it into the project (as
  *Tools*, tool-per-operation.md §8) and adds the pass: one undo step. When no tool is
  smaller, or the pocket has dogbones, the entry is still shown, greyed out, and its hover
  says why (changed in 0.22.1: before, it was left out, and users couldn't find it).
  Right-click a pass: *Remove*, on/off.
- **Properties:** a pass shows its own fields, and its pocket's (shapes, depths) as read-only
  facts with a link to the pocket.
- **CLI:** `chipload op add-rest part.chipload --op 1 --tool 2` adds a pass (as now, but
  under the pocket); `op rest-pass` lists, sets and removes passes; `op rest` keeps setting
  a pocket's standalone `rest`.
- **Setup sheet:** each pass a row under its pocket's, in cut order, "rest of 2, T2 (3 mm)".

## 7. Checks

- **Problems:** a pass's tool isn't in the project, isn't an endmill, or isn't smaller than
  the tool before it; the pocket has dogbones (as rest-machining.md §5).
- **No warning needed** for a missing earlier pocket: a pass has one by construction.

## 8. Tests

- A pass follows its pocket: change the pocket's shapes and depths, the pass cuts the new
  ones; turn the pocket off, the pass isn't cut.
- The order rule: the table in §3, with the G-code's tool changes counted; a pocket alone,
  a run of two with one pass each, and a chain of two passes.
- Same G-code: a pass gives the same moves as today's rest pocket with the same values,
  only (possibly) at another place in the order.
- The file: saved without `rest_passes` when empty; the corpus is unchanged (no golden
  changes); a new golden for the two-pocket run.
- *Make it a rest pass*: offered only for a matching rest pocket, and the result cuts the
  same moves.

## 9. Decisions

1. **A rest pass lives inside its pocket** and takes the pocket's shapes, depths, clearance
   and direction. **Recommended**: it can't drift, and the Browser shows what belongs
   together. Alternative: keep today's copied rest pocket and only draw it indented.
2. **Cut after the run of same-tool pockets** (§3). **Recommended**: one tool change for the
   rests of all those pockets, and the outline still last. Alternatives: right after its
   own pocket (simplest, most tool changes), or after everything in the part with the
   large tool (fewest changes, but it can put a rest after the outline, when the part
   already hangs on its tabs).
3. **Several passes per pocket**, smaller and smaller. **Recommended**: a 12 mm, 6 mm, 3 mm
   chain is common for deep corners, and it falls out of the same computation.
4. **An optional field, no schema bump**, as `drawing.texts`. **Recommended**: nothing
   changes for files without passes.
5. **Keep the standalone rest pocket**, don't migrate, and offer *Make it a rest pass* where
   one matches. **Recommended**: it serves the pocket-cut-elsewhere case, and a migration
   that moves cuts in the order would change G-code silently, which CLAUDE.md forbids.
6. **Pockets only**, as today. Rest passes for profiles can follow the same way if asked
   for.
