# Spec: exporting one part's G-code

Status: **approved 2026-10-29** ("sounds good go ahead": all five as recommended) and
**built** the same day. As built, the window plans the part in the background as its
preview does, so the toolpath shown is that part's afterwards (§4 said it wouldn't change);
the status while planning names the part, and the export's own status is as for *Export*.

Follows docs/specs/parts.md and docs/specs/active-part-preview.md, whose §2.2 ("Export
always writes every part that is cut") this changes: *Export* still does that, and a new
command writes one part.

## 1. Why

The user (2026-10-29, after the first test cut on the Shapeoko): "We should be able to right
click on a part and choose export gcode to get the gcode for only that part even if there are
many parts in the project."

A project often holds several parts that are cut on different days, from different boards,
or again after a broken one. Today that means switching the other parts off, exporting, and
switching them on again. That is three steps, and the state of the file changes each time.

## 2. What "one part's G-code" is

**Exactly the program the project gives with every other part switched off**, nothing more:

- the part's operations in their order, with all its copies (grid);
- the same stock, work zero, machine, post-processor and tool change setting;
- one file per tool or one program with tool changes, as the project says;
- checked by the independent checker, as every program is (`run_job` refuses otherwise);
- the project file itself is not changed.

This is what the window's preview of the active part already plans
(active-part-preview.md §2.1), so the planning is not new; only writing it is.

## 3. Command line (first, CLAUDE.md rule 5)

```
chipload run box.chipload -o lid.nc --part 2      # only part 2 (as `part list` numbers it)
chipload sheet box.chipload -o lid-sheet.html --part 2
chipload plan box.chipload --part 2 --json
```

- `--part ID` may be given more than once: those parts, in the project's order.
- An unknown ID is an error that names the parts there are.
- Without `--part`, as today: every part that is switched on.

## 4. The window

- **The part's right-click menu** in the Browser gets **Export G-code of this part…**, after
  *Activate*. It opens the save dialog, with the name suggested as decision 3 says.
- Planning happens in the background as for *Export*: if the toolpath shown is already
  exactly this part's, it is written at once; otherwise the part is planned first, and the
  status says "Planning part "lid", then exporting".
- The **setup sheet**, when the setting writes one, is of that part only: its operations,
  its copies, the stock and zero. Its title names the part: "Setup sheet: box.chipload,
  part "lid"".
- The status after writing says which part: "Exported part "lid" to lid.nc".
- The toolpath shown and the preview scope don't change.

## 5. Tests

- `run --part 2` on a two-part project gives byte-identical G-code to `run` on the same
  project with part 1 switched off (`--set parts.0.enabled=false`).
- `--part` twice; an unknown ID; a one-part project with `--part 1` equals `run`.
- A part with a grid of copies: all copies are in its program.
- Window: the menu item exports the part, the file equals the CLI's; the project is unchanged
  and not marked as edited; with the preview showing that part it writes without planning
  again.
- The sheet names the part. Six languages for the new texts; the menu in a narrow window.

## 6. Decisions

1. **A switched-off part can still be exported this way (recommended).** The menu says
   which part, so the choice is explicit. Or: the item is greyed out while the part is off.
2. **`--part` on `run`, `sheet` and `plan` (recommended)**, repeatable. Or `run` only.
3. **Suggested file name: the project's name and the part's**, e.g. `box-lid.nc`
   (recommended), with the part's name made safe for file names. Or the part's name alone
   (`lid.nc`), or the last export folder and name.
4. **The G-code itself names nothing new (recommended):** no extra comment line for the
   part, so the program stays exactly what switching the others off gives, and no post
   template changes. The setup sheet and the status say which part it is. Or: a comment
   line "(part: lid)" in the header, which would change every post's template and golden
   files.
5. **Also in the part's Properties?** A button *Export G-code of this part* there as well
   (recommended: no, the right-click menu only, to keep Properties for settings). Or both.
