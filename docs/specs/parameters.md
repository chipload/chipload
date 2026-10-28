# Spec: parameters (drawing phase 4)

Status: **draft 2026-10-28, on hold** (the user, the same day; decisions in §11 not yet
reviewed). The user chose designs (a box
maker, docs/specs/designs.md) to come first; parameters follow and can drive a design's
values.

Phase 4 of docs/specs/drawing-roadmap.md (§7). Project values with names, used in the
length fields of shapes and operations, so that one change carries through the drawing and
the depths:

```
thickness = 18
slot_w    = thickness + 0.2
```

A rectangle's width `slot_w`, a pocket's end depth `thickness / 2`: change `thickness` to 19
and the slot widens and the pocket deepens. This is the leverage version of "parametric":
no constraint solver (out of scope, ADR 0017), only values and arithmetic.

## 1. What a user can do

- **Define parameters:** a name and an expression, such as `thickness = 18` or
  `slot_w = thickness + 0.2`. Each shows its value.
- **Use them in a field:** type `slot_w` or `thickness / 2 + 1` where a length or a count
  goes, in the window or on the command line. The field keeps the expression and shows its
  value.
- **Change one** and everything that uses it follows at once, as one undo step. The status
  bar (and the CLI) says how many fields changed.
- **See what uses a parameter,** and turn a field back into a plain number.

## 2. Expressions

- **Numbers** with `.` as the decimal point (a `,` is read as one too, as in the window's
  fields), optionally with a **unit**: `mm`, `cm`, `in`. `1/2in` reads as `1/2 in`, i.e.
  12.7 mm.
- **Operators** `+ − × ÷` written `+ - * /`, unary minus, and parentheses, with the usual
  precedence.
- **Parameter names:** a letter or `_`, then letters, digits or `_` (`slot_w`, `T2`);
  case-sensitive. `mm`, `cm` and `in` can't be names.
- **No functions** (min, max, round) and no conditions in this phase.
- **A bare number is millimetres** in the file (decision 4); `0.75in` is 19.05 mm.
- **Units are scales, not checked dimensions:** `2in * 3` is 152.4; `thickness * 2` is a
  length; nothing refuses `thickness * thickness`. The result is used as the field's number.
- **Refused, as problems:** an unknown name, a cycle (`a = b + 1`, `b = a`), division by
  zero, a result that isn't a finite number, a count that isn't a whole number (within
  10⁻⁹), and anything the field itself refuses (a negative radius, a depth past its limit),
  with the field's own message.
- **Hand-written parser** in `chipload-job` (a small recursive descent over these tokens);
  no dependency. Evaluation is plain `f64` arithmetic, so the same project gives the same
  numbers, and the same G-code, everywhere.

## 3. Where they live (decision 1)

- **Parameters:** a new top-level list, `parameters: [{"name": "thickness", "expression":
  "18"}, …]`, in the order the user made them. They may use each other in any order
  (decision 3); the value of each is worked out from what it uses.
- **Uses:** the field keeps its **number**, and the object it belongs to keeps the
  **expression** beside it, in an `expressions` map from the field to the expression:

  ```json
  {"id": 4, "kind": "rectangle", "length": 120.0, "width": 18.2,
   "expressions": {"width": "slot_w"}, …}
  {"type": "pocket", "end_depth": 9.0, "expressions": {"end_depth": "thickness / 2"}, …}
  ```

  Every reader of the number (planning, the checker, the setup sheet, older scripts) is
  unchanged: the number is always there and always right. The expression travels with its
  object when operations are reordered, shapes copied or moved to another part.
- **Keys** are the field's path inside its object, as `get`/`set` paths: `width`,
  `center.0`, `tabs.height`.
- **Schema 32:** `parameters` and `expressions` are new; migration 31→32 adds an empty
  `parameters` list and changes nothing else (goldens unchanged). An older Chipload refuses
  the file rather than dropping the expressions on its next save.

## 4. Which fields take an expression (decision 2)

Lengths and counts, the numbers a design is made of:

| Where | Lengths | Counts |
|---|---|---|
| Drawn shapes | circle centre and radius; rectangle centre, length, width, corner radius; polygon centre, radius; polyline points | polygon sides |
| Texts | height, position, letter spacing | |
| Operations | start and end depth, clearance, step down, tab width and height, finishing and skin allowances, hole diameters, V-carve depths | tab count |
| Stock | width, height, thickness, allowed overcut | |
| Drawing files | position (move) | |
| Copies | gaps and pitch | columns, rows |

The exact list is a table in the code (`params::BINDABLE`), checked against the JSON
schema by a test so a new length field is either added to it or left out on purpose.
**Not in this phase:** angles (stored as radians in some fields and degrees in others),
feeds and speeds, ratios such as stepover, and tool sizes (the tool library's own).

## 5. When values are worked out

- **On every change** to a parameter or an expression (the command line, the window), all
  parameters are evaluated, then every expression, and the results written into their
  fields; then the project is validated as always. One undo step.
- **Never silently on load** (decision 5): a project whose stored number differs from what
  its expression gives (a hand edit of the file) has a **problem** on that field: "18.2,
  but its expression `slot_w` gives 19.2". `check-project` and the window show it, and `run`
  refuses, until `chipload param apply` (*Recompute* in the window) writes the expression's
  value, or the field is set to a number, which drops its expression.

## 6. Changing a field that has an expression (decision 6)

- **Typing a number** in it drops its expression: it becomes a plain number again.
- **Typing an expression** replaces the old one.
- **Dragging a handle,** moving, turning or scaling a shape, or an editing operation that
  changes a bound value directly, also drops that value's expression. The status bar says
  which ("S4's width no longer follows `slot_w`"); undo brings it back.
- **A shape that changes kind** (a rectangle filleted into a polyline, a union) loses the
  expressions of fields it no longer has, with the same note.
- **Exploding a DXF or a union** makes shapes without expressions (they had none).

## 7. Command line

```
chipload param list P                               # name, expression, value, uses
chipload param set P thickness=18 'slot_w=thickness + 0.2'   # add or change; recomputes
chipload param remove P slot_w                      # refused while used (--keep-values)
chipload param use P drawing.shapes.3.width 'slot_w'           # an expression in a field
chipload param use P operations.1.end_depth 'thickness / 2'
chipload param clear P drawing.shapes.3.width       # back to a plain number
chipload param apply P                              # recompute after a hand edit
chipload run P --param thickness=19 -o out.nc       # for this run only, the file unchanged
```

- `chipload set` keeps setting plain values; setting a field that has an expression drops
  the expression, as typing a number does in the window (§6), and says so.
- `run`, `plan` and `sheet` take `--param NAME=EXPR` for one run, so a script can try
  thicknesses without changing the file (as `--set` does for values).
- `--json` gives each parameter's value and its uses.

## 8. The window

- **Job settings** gets a **Parameters** section: a row per parameter (name, expression,
  value), *Add*, and a remove button that is disabled while it is used (its hover lists
  the uses). A value that can't be worked out shows the problem in the row.
- **Fields that take an expression** accept one when typed: anything that isn't a plain
  number. A field with an expression shows its value with a small **ƒ** marker; hovering
  shows the expression; focusing it edits the expression.
- **In inch display**, a bare number typed into an expression gets `in` written after it
  (`thickness + 0.01` becomes `thickness + 0.01in`), so the file always means what was
  typed (decision 4).

## 9. Edge cases

- A parameter that nothing uses is fine. A use of a parameter that was removed with
  `--keep-values` becomes its number.
- Renaming a parameter renames it in every expression that uses it (`param rename`, and
  in the window).
- Copies and arrays of shapes keep the expressions; pasting into another project keeps
  them only when that project has the parameters, otherwise the numbers (with a note).
- A text's height bound to a parameter re-letters the text when it changes, as editing the
  height does.

## 10. Tests

- The parser and evaluator: precedence, units, unary minus, commas, errors with positions,
  cycles, any order of definition; a proptest that an expression printed and parsed again
  gives the same value.
- The pipeline: a change carries to shapes, texts, operations and the stock; a mismatch is
  a problem and `run` refuses; changing a bound field drops its expression; kind changes
  drop theirs.
- CLI: each command and its refusals; `run --param` leaves the file alone.
- Migration 31→32 on the frozen v31 sample; goldens unchanged. One new golden: a parametric
  part (a slot sized from `thickness`, pocketed `thickness / 2` deep), run twice with
  `--param thickness=` two values, both checked, the slot and depth following.

## 11. Decisions

1. **Numbers stay in their fields, expressions beside them** in each object's
   `expressions` map, rather than fields that hold a number or an expression.
   *Recommended: yes*: every reader of the file keeps working on numbers, and nothing in
   planning or checking changes.
2. **Lengths and counts only** (§4), not angles, feeds, speeds or ratios, in this phase.
   *Recommended: yes*: they are what a design is made of, and angles need their units
   sorted out first.
3. **Parameters may use each other in any order;** cycles are refused. *Recommended:
   yes*, rather than only earlier ones (the roadmap's "in order"): a user adds parameters as
   they think of them.
4. **A bare number is millimetres in the file;** the window in inch display writes `in`
   after numbers typed without a unit. *Recommended: yes*: the file must mean the same
   whatever display a reader uses.
5. **A stored number that differs from its expression is a problem,** fixed by `param
   apply`, never recomputed silently on load. *Recommended: yes*: a project must never cut
   differently from what it says without a word.
6. **Changing a value directly drops its expression** (with a note, undoable), rather than
   locking fields that have one. *Recommended: yes*: dragging a handle should just work.
7. **Removing a parameter in use is refused** unless `--keep-values` (the window: a
   question). *Recommended: yes.*
