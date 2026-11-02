# Spec: explode a picture or a text into drawn shapes

Status: **agreed 2026-11-02**, decisions 1-5 as recommended (the user: "looks good go
ahead"). Built 2026-11-02.

As built:
- **The G-code is the same but for rounding.** A letter's or a traced outline's arcs come
  back from the polylines' bulges a few billionths of a mm off. That can tip a V-carve's
  depth to the next 0.001 mm: one line in the corpus, `v29/script_text`. The corpus test
  allows a last-digit difference in numbers and nothing else.
- **One `explode` in the pipeline** serves DXFs, pictures and texts, generalised over
  `ShapeFrom`. The DXF's join stays its own.
- **The dialog** is the DXF's, with the body for a picture or a text and without the join
  row.

The user (2026-11-02): "i cant explode and image or text to shapes only an dxf, is that by
design?" It isn't: *Explode* (docs/specs/explode-and-join.md) was asked for DXFs and
built for them alone. This extends it to the other two kinds of shapes that are made from
something: a traced picture's and a text's.

## 1. Today

- **A traced picture** (`drawing.images`) keeps the picture itself and its trace settings.
  Its shapes are `path` shapes with `origin: image`. They are traced again whenever a
  setting changes (images.rs), and they are read-only: no handles, no editing one by one.
  This applies to outlines and to centre lines alike.
- **A text** (`drawing.texts`) keeps its words, font, height and place. Its letters are
  `path` shapes with `origin: text`. Overlapping outlines are already merged when it is
  lettered (texts.rs, more-fonts.md §4). They are lettered again whenever the text
  changes, and they are read-only too.
- **Explode a DXF** turns its `path` shapes into drawn polylines and circles
  (`path_to_drawn`), keeping ids and layers. It removes the DXF, and rewrites selectors
  limited to that DXF into lists of ids, so the G-code stays the same
  (explode-and-join.md §2).
- **The shape library** already saves a picture's shapes as drawn polylines and circles
  through the same `path_to_drawn`. A text's letters are kept there as paths with their
  text (shape-library.md, As built).

## 2. Explode a picture

*Explode* turns **every shape of one picture** into drawn shapes. It removes the picture
from the project: its entry in `drawing.images` with the embedded picture file, and its
hidden-group keys `image:N…`.

- **Shapes** become drawn shapes by exactly the DXF's rules (explode-and-join.md §2):
  - a full circle becomes a `circle`;
  - anything else becomes a `polyline` with bulges;
  - a path in several runs becomes one polyline per run, with new ids after the first;
  - segments shorter than `CHAIN_TOL_MM` are left out.

  Traced outlines become closed polylines, and centre lines open ones.
- **Ids and layers** stay, so operations that list the shapes keep cutting them.
- **Selectors limited to the picture** (`{"image": N}`, with or without `layer` and
  `exclude`) become the ids they matched, as a DXF's do.
- **What goes:**
  - the picture on the canvas under its shapes;
  - *Trace settings…*, so it can't be traced again;
  - the picture file inside the project, which makes the project smaller.

  Undo brings it all back.

## 3. Explode a text

*Explode* turns **every letter of one text** into drawn shapes. It removes the text from
`drawing.texts`, and its hidden-group keys `text:N…`.

- **Letters** become drawn polylines by the same rules. A letter's outline and its holes
  (the inside of an O) are separate shapes already, and stay so.
- **Ids and layers** stay. Selectors limited to the text (`{"text": N}`) become the ids
  they matched.
- **Groups:** a text in a group (`groups[].texts`) is replaced there by its letters' ids
  (`groups[].shapes`). The group keeps the same shapes, and its selectors keep cutting
  them.
- **What goes:**
  - editing the words, font and height;
  - the text's box handles. The letters can still be moved, turned and scaled as drawn
    shapes, with the box round the selection.

  Undo brings it back.

## 4. The G-code stays the same

The same geometry in the same order, as for a DXF. A project's program is the same before
and after exploding any of its pictures or texts. §7 tests this on the corpus.

No schema change: the result is ordinary drawn shapes.

## 5. Command line (first, rule 5)

```
chipload image explode PROJECT [ID] [-o OUT]
chipload text explode PROJECT [ID] [-o OUT]
```

- These are beside `image add/set/list/remove` and `text add/set/list/remove`, as
  `dxf explode` is beside `dxf add/list/remove`.
- With one picture (or text) in the project, no ID is needed. With several, the error
  lists them, as `dxf explode` does.
- `--json` gives the new and removed ids and the rewritten selectors.
- MCP gets both as tools, as it gets every command.
- No `--join` (decision 4): *Join ends* (`shape join`) is there for the rare case.

## 6. App

- **Browser:**
  - a picture's context menu gets *Explode into shapes…* between *Trace settings…* and
    *Remove*;
  - a text's gets it above *Remove*.
- **Canvas:** right-clicking with a whole picture, text or DXF selected also offers
  *Explode into shapes…* (decision 3).
- **A small dialog says what is lost:**
  - for a picture, "The shapes become drawn shapes you can edit. The picture itself goes
    from the project: it can't be traced again.";
  - for a text, "The letters become drawn shapes you can edit. The words can't be edited
    again.".

  It has *Explode* and *Cancel*, and no join field.
- The new shapes are selected, and appear among the drawn shapes in the Browser
  (*Polylines*, *Circles*) on their layers.
- One undo step.
- Every new text in all six languages.

## 7. Where the code lives, tests

- **Code:** `chipload-pipeline/src/explode.rs` gets `explode_image` and `explode_text`.
  - They share the DXF's loop over shapes and its selector rewriting, generalised over
    `ShapeFrom` (Dxf, Image, Text).
  - The report's messages are reused where they fit ("exploded", "split", "dropped",
    "selects"), with the kind named.
  - `chipload-job` doesn't change.
  - The guide pages are *Drawings > Pictures* and *Drawings > Text*.
- **Tests:**
  - **The G-code stays the same:** every corpus project with a picture or a text, each one
    exploded, gives the same G-code as before (golden comparison, no new goldens).
  - **Pictures:** outlines become closed polylines and centre lines open ones. Points are
    within `CHAIN_TOL_MM` of the original. The picture and its hidden keys are gone.
  - **Texts:** a letter with a hole stays two shapes. A text in a group is replaced by its
    letters, and the group's operation cuts the same.
  - **Selectors:** `{"image"}` and `{"text"}`, with `layer` and `exclude`, are rewritten
    to ids. Selectors listing ids are unchanged.
  - **CLI:** both commands, with and without `--json`; no ID with one and with several.
  - **App:** the Browser menu and the canvas menu each explode, as one undo step.

## 8. Decisions to review

1. **The whole picture or text, not single shapes.** *Recommended:* as for a DXF, a
   picture or text is either linked or exploded, never half. *Instead:* explode the
   selected shapes only, leaving a picture with holes in its trace.
2. **The picture is removed with its trace.** *Recommended:* nothing would use it. A
   picture kept only to show on the canvas would be a new kind of thing, with no trace
   and no shapes. *Instead:* keep it as an untraced underlay to draw over. That would be
   a feature of its own, later if wanted.
3. **Also in the canvas's right-click menu, for DXFs too.** *Recommended:* that's where you
   looked for it, and the selection is already there. *Instead:* in the Browser only, as
   for DXFs today.
4. **No join option.** *Recommended:* traced outlines and letters are closed already. A
   centre-line trace's open ends meet at junctions where three or more ends meet, which
   join leaves alone. *Instead:* the DXF dialog's *Join ends closer than* field here too.
5. **No schema change, nothing to migrate.** *Recommended:* the result is drawn shapes the
   current format already has.
