# Fewer clicks for the usual job

Status: **approved 2026-11-14**, every decision as recommended (TODO *Next* item 1, the
user: "prioritize usability improvements and a streamlined, quick and efficient
workflow"). §5 built first, then §2, then §3, then §4.

## 1. The usual job today

A part from a DXF, in the window, as 0.36 has it:

1. **Open** the DXF (Ctrl+I, or drop it on the window).
2. **Select** what one operation cuts: click, Ctrl+click, a box, or right-click for a
   layer.
3. **Add** the operation: a ribbon button, the canvas's right-click menu, or a style
   (*Style ▾* or the right-click menu's *Apply style ▸*). Defaults and styles (0.33) fill in
   side, depth, through, tabs and feeds.
4. Repeat 2 and 3 for each operation (often three: holes, pockets, the outline).
5. **Generate:** nothing to do, as *Generate automatically* is on by default.
6. **Export:** Ctrl+E, then the file dialog every time, which suggests a file next to
   the project.

What costs clicks: in step 2, picking many small shapes one by one (every 5 mm hole, every
pocket inside the outline); in step 3, the trip to the ribbon or a two-level menu; in step
6, the dialog. A user who cuts the same kinds of parts again and again repeats steps 2-4
the same way each time; layer rules do them in one go.

## 2. A quick menu at the cursor: Q

*Changed 2026-11-15:* the menu moved to **S**, Q now shows the Compass
(docs/specs/compass.md §9); the slot's key S became U.

**Q** over the canvas, with shapes selected, opens a small menu at the mouse:

```
Add for the selection
 P  Profile
 K  Pocket
 H  Holes
 V  V-carve
 F  Follow path
 ─────────────
 1  Cut out (style)
 2  Drill 5 (style)
 3  Engrave (style)
```

- A letter or a digit chooses at once, and so does a click. Escape or a click elsewhere
  closes the menu. The menu's own letters work only while it is open, so they don't
  clash with the drawing keys (P is *Panels* and C *Circle* outside it).
- **Digits 1-9 are the styles,** in the Styles list's order, so reordering the list
  changes the digits. Styles past the ninth are listed without a digit.
- Kinds the selection can't take are greyed out with the reason as a hover (holes without
  circles, a V-carve without closed shapes). Operations come out exactly as the ribbon
  button or the style makes them, through the same `new_operations` (styles.md §2).
- With nothing selected, Q says so on the status bar ("Select shapes first, then Q").
- The keyboard shortcuts window (F1) and the right-click menu name Q.

## 3. Selecting by kind

Two items in the right-click menu of a selected shape. Each replaces the selection; with
Ctrl held, it adds to it.

- **Select similar:**
  - a circle selects every circle of the same diameter, within the chaining tolerance,
    on any layer: the holes of one size;
  - any other shape selects the shapes of the same kind (closed or open) on the same
    layer whose boxes are the same size, within the chaining tolerance, either way round:
    the same slot or pocket repeated.
- **Select inside:** a closed outline selects every closed shape inside it: the pockets
  and holes of one part, without the outline. Nested shapes count too (an island's hole).

Command line first (architecture rule 5): `chipload shape similar PROJECT ID` and `chipload
shape inside PROJECT ID` print the ids (and `--json` gives them as a list). An operation
then takes them with `op add … --shapes 4,7,9`. Both live in the pipeline
(`selection::similar`, `selection::inside`), which the window calls.

## 4. Layer rules

A style can have a **layer pattern**: `CUT*`, `DRILL_?`, `pocket`. The rules are
presets.md §5, as decided there, carried over to styles:

- **Pattern:** `*` for any run of characters and `?` for one, upper or lower case alike, as
  layer selection already is. No regular expressions.
- **Order:** the Styles list's order is the order the rules add operations. Within one
  style, the layers go by name. Every style that matches a layer applies, so a rough pass
  and a finish pass can both come from rules.
- **Selector:** the operation selects the layer (`select.layer`), so an updated drawing
  stays in step.
- **Already cut:** a layer that an operation already cuts, by layer or by one of its
  shapes, is skipped with a note. Applying twice adds nothing.
- **When:**
  - on import, when any style has a pattern (`chipload import … --no-rules` skips them;
    one Undo in the window);
  - on demand: *Operation > Apply layer rules*, or `chipload style apply PROJECT
    [--dry-run]`;
  - not on update: `update` names new layers that match a rule.
- **The report:** one line per operation added, one per layer skipped and why, and the
  layers no rule matched, the same on the command line and in the window.

In the window the pattern is a field in the style's properties (*Layers*), empty for none.

**Format:** `styles.json` gains an optional `layer` per style, left out when empty, so
schema version 1 stays. As with `materials.json`'s `default_thickness` in 0.33, a Chipload
older than this refuses a styles file that has a pattern. Its styles stay on disk, and
nothing machines differently.

## 5. Export with one key

As Save and Save as work:

- **Ctrl+E**, the first time for a project, asks where to write, as today. After that it
  writes to the same file again without asking, and the status bar says "Exported
  /path/to/part.nc", as every export does. The setup sheet beside it is written again
  too, when that setting is on.
- **Ctrl+Shift+E**, *File > Export G-code as…*, always asks.
- The file is remembered per project in the app settings, for the projects in the
  recent-projects list: `exports`, a map from the project's path to the G-code file's,
  left out while empty, so settings stay version 3 (a Chipload older than this starts with
  default settings once this one has saved one, as for `new_project`). A project that
  leaves the recent list forgets its file. An untitled project always asks. A remembered
  file whose folder is gone asks again, and the dialog suggests the remembered file.
- *File > Export G-code* loses its "…", as Save has none; the menu gains *Export G-code
  as…*.
- Export by part or by setup (part-export.md) keeps asking: each is a different file.

## 6. Not here

- **Styles of several operations** (a profile and its pocket as one style): their own
  spec later, once layer rules show whether they are still needed.
- **A key per style outside the Q menu:** the menu's digits do it in two keys without
  taking more of the keyboard.

## 7. Order of building

1. Export with one key (§5).
2. The Q menu (§2).
3. Select similar and select inside (§3), command line first.
4. Layer rules (§4), command line first.

Then the walk again (TODO *Next* item 3): count the edits and clicks for the usual jobs.

## 8. Tests

- Export: the first Ctrl+E asks; the second writes the same file without asking; Export
  as asks; untitled asks; a missing folder asks; the setting round trip.
- Q menu: each letter and digit adds what the ribbon button or style adds (the same
  operations); greyed kinds; Escape; nothing selected; layout in every language.
- Similar: circles by diameter across layers; slots by box size either way round; open
  vs closed; Ctrl adds. Inside: nested shapes, the outline excluded, open shapes ignored.
  The command line gives the same ids as the window.
- Layer rules: pattern matching (case, `*`, `?`); order across and within styles;
  already-cut layers skipped; applying twice; on import with `--no-rules`; `--dry-run`;
  the report lines; a styles file with patterns round trip; one Undo.
- Golden: a corpus DXF imported with two layer rules gives the same G-code as the same
  operations added by hand.

## 9. Decisions

1. **A Q menu at the cursor** (§2) rather than one direct key per kind. *Recommended:*
   the drawing tools already use most letters, and a menu that lists its keys teaches
   itself. The alternative, Shift+letter per kind, is one key shorter but has to be
   learnt.
2. **Styles by digit, in list order,** inside the Q menu: no new field. *Recommended.*
3. **Select similar** by circle diameter, or else by kind, layer and box size; **Select
   inside** for an outline's inner shapes (§3). *Recommended.*
4. **Layer rules as styles with a pattern** (§4), presets.md §5's rules, on import and on
   demand, not on update. *Recommended.*
5. **Ctrl+E like Save** (§5): asks once per project, then writes to the same file;
   Ctrl+Shift+E always asks. *Recommended.*
6. **Styles of several operations later**, in their own spec (§6). *Recommended.*
