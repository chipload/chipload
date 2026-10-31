# Spec: frames and a library of shapes (for signs)

Status: **agreed 2026-10-31**, all decisions as recommended (the user: "Go ahead with
recommendations we will iterate a bit on it"). **Step 1, frames (§3), built the same day**;
step 2, the library (§4), next.

As built (frames):
- *r* is called the **corner size** (`--corner` on the command line, *Corner* in the
  window). Unset, it is a tenth of the shorter side of the frame being drawn (for a frame
  round shapes: of the box plus margin and borders).
- The shapes get **no names** (§3.4 said *Frame*, *Border 1*): a name stored in the
  project would be in one language whatever the window's; the layers tell them apart.
- The **window** has no separate dialog (§5): *Draw > Frame…* is a tool like *Array*, its
  values under *Selection* with the frames as small pictures, a live preview on the canvas,
  *Add* and *Cancel*. Clicks while it is open change what it goes round; with nothing
  selected it draws to a size, centred on the stock. After *Add* the frame is selected.
- A frame round a box grows **just enough**, found by halving (`fit_round`): equally in
  both directions for the rectangular frames, in proportion for the oval, in height for the
  arched top, along the long side for the plaque. The arched top's box corners rest on its
  straight sides below the arch.
- The **oval** is the ellipse sampled closely where it curves tightly (chords within a
  tenth of the arc tolerance) and fitted with arcs (`fit_path`) within the rest: 34 pieces
  for a 300 × 120 oval. A round oval is a circle shape.
- **Borders** are the offset of `drawing_edit` (checked as every offset); an inward offset
  that splits in two is refused as a border.
- **Cutting a sign** (§6): the border is grooved by a **follow path** with the V-bit; a
  V-carve of a closed border would carve the whole area inside it. The golden
  `v35_sign_frame` and the guide's page *Signs and frames* do so.

The user (2026-10-31): "One big use case is making engraved signs. I wonder if we can
simplify this work. Perhaps a library/catalog of frames where the user can quickly insert a
frame and also save frames or shapes for future use."

## 1. Why

Making a sign today looks like this:
1. Write the text.
2. Draw a rectangle round it by eye.
3. Round its corners.
4. Offset it inwards for a border line.
5. Centre everything by hand.

Anything fancier than a rounded rectangle, such as an oval, an arched top or notched
corners, has to be drawn in another program and imported. A drawing that worked well, like
the user's own house-number plate or a logo with its border, has to be copied out of an old
project each time.

This spec adds two things:
- **Frames:** a few built-in sign outlines, drawn to a size or round the selected shapes,
  with border lines. Section 3.
- **The shape library:** any selected shapes (and texts) saved under a name in the settings
  folder, and inserted into any project later. Section 4.

Both put ordinary drawn shapes into the project. Neither needs a new project format.

Clean room (CLAUDE.md): the frame outlines are plain geometry worked out here (rectangles,
arcs, offsets). None is taken from another program's catalogue or sample files.

## 2. What a user can do

- **Frame a text:**
  1. Select the text (or any shapes).
  2. Choose *Draw > Frame…*.
  3. Pick a frame and set the margin and the border lines.

  The frame is drawn round the selection, centred on it.
- **Draw a frame to a size:** the same with nothing selected. It has a width and a height
  and is placed at the centre of the stock. That's a sign blank.
- **Save to library:** select shapes and texts, right-click, *Save to library…*, and give a
  name. They are kept in the settings folder, so every project on this computer can use
  them.
- **Insert from the library:** the Library's new **Shapes** tab shows each entry as a small
  picture with its name. Clicking *Insert* adds the shapes to the project:
  - at the centre of the view, selected, so they can be dragged where they belong;
  - or, with *Fit to selection*, scaled to fit round what is selected.
- **Rename, delete, share:** entries are renamed and deleted in the Shapes tab. Because each
  entry is one file, it can be copied to another computer or sent to a friend (§4.2).
- **The same on the command line** (§7), as every feature (CLAUDE.md rule 5).

## 3. Frames

### 3.1 The built-in frames (decision 2)

Each frame is one closed outline, drawn from a width and a height:

| Frame | Outline |
|---|---|
| Rectangle | sharp corners |
| Rounded | corners of radius *r* (default: 10 % of the shorter side) |
| Notched | each corner cut in by a quarter circle of radius *r*, curving inwards |
| Chamfered | each corner cut off at 45° by *r* |
| Oval | an ellipse through the box's four sides, drawn as arcs within the arc tolerance (`chipload_core::tolerances`) |
| Arched | a rectangle whose top side is an arc rising by *r* above the corners |
| Plaque | a rectangle whose short sides bulge outwards as arcs of height *r* |

- A circle or a regular polygon frame is the oval with width = height, or the existing
  polygon shape. Neither is a frame of its own.
- *r* is one length, with the meaning shown in the table. It is checked so that the outline
  stays a simple closed loop that doesn't cross itself. Otherwise it is refused with the
  largest value that works.
- More frames can be added later. Each one is a function from width, height and *r* to a
  closed outline, with its tests and its texts, and nothing else changes.

### 3.2 Border lines

- **Borders:** 0 to 3 extra lines inside the outline, each offset inwards from the one
  before by **border gap** (default 5 mm). The offset is `chipload_draw::offset`, so the
  borders keep the outline's character: corners stay rounded or notched as they should.
- The outline and each border are separate drawn shapes. A typical sign:
  - V-carves or engraves the border line;
  - profiles the outline to cut the sign out.

### 3.3 Size and placement

- **Round the selection:** the box round the selected shapes, plus **margin** (default
  10 mm) on every side, is the space inside the **innermost** border. The text gets its
  margin from the line nearest to it, not from the outer edge. Width and height follow
  from that and are shown. The frame is centred on the box.
- **To a size:** width and height are of the outline, and the frame is centred on the
  stock.
- An oval or a plaque round a box needs more room than a rectangle to clear its corners.
  It is grown just enough that the box fits inside its innermost line with the margin.
- **Layers** (decision 5): the outline goes on the layer chosen in the dialog (default
  `frame`), the borders on `border`. They are ordinary drawn shapes; their layers can be
  changed afterwards.

### 3.4 In the project

- A frame becomes ordinary drawn shapes (closed polylines with bulges, or a rectangle shape
  for *Rectangle* and *Rounded*), named *Frame* and *Border 1*, *Border 2*, ….
- **No format change.** No frame kind or values are stored (decision 3). To change a
  frame, draw it again: the dialog opens with the values last used in this session.
- One undo step for the whole frame.

## 4. The shape library

### 4.1 What an entry holds (decision 4)

- **Drawn shapes**, with layer, name and the construction flag.
- **Texts**, which stay texts: words, font, height, alignment, with their letters. A text
  whose font isn't on the computer it is inserted on keeps its letters and is locked, as in
  docs/specs/system-fonts.md.
- **Imported shapes** (from a DXF, an SVG, a picture) are saved as drawn path shapes. Their
  link to the file is not kept.
- **Not operations, tools or stock.** An entry is drawing only (decision 4; operations
  belong to the styles and presets that are paused).
- Positions are kept relative to the entry's own box. On insert, the box's centre goes to
  the chosen point.

### 4.2 Where it is kept: one file per entry (decision 6)

- **Location:** a `shapes` folder in the settings folder, beside `posts/` and `fonts/`.
- **One file per entry:** `<name>.chipload-shapes`, JSON.
  - It holds `schema_version`, `written_by`, `name`, `shapes` and `texts`, in the
    project's own shape and text format.
  - It is a **persisted format of its own**, with `SHAPES_VERSION` and migrations, loaded
    through one `load_shape_entry` that refuses missing or newer versions.
  - When the project's shape or text format changes, this format gets its version bump and
    migration in the same commit (CLAUDE.md, persisted formats).
- **DXF and SVG files** put in the same folder are listed too, and inserted through the
  importer, with their import report (decision 7). Their units must be known: a DXF without
  `$INSUNITS` is listed but refused on insert with the usual advice.
- **Name:** the file name without its extension. Renaming an entry renames its file.
- **Thumbnails:** drawn from the geometry when the tab opens, never stored.
- **Errors:** an unreadable or too-new file is listed with its error and can't be inserted.
  It is never deleted or overwritten.

### 4.3 Saving

- **What is saved:** the selected shapes and texts. A text's letters are saved with it,
  never as loose letters.
- **Same name:** if an entry of that name exists, the dialog asks before overwriting it,
  and keeps the old file as `.bak` (as `save_job` does).

### 4.4 Inserting

- **New ids:** shapes and texts get new ids in the project; nothing in the project
  changes.
- **Layers:** each shape keeps its layer from the entry. On the command line, `--layer`
  puts everything on one layer.
- **Scale:** *Fit to selection* (or `--around IDS --margin`) scales the entry uniformly so
  that its box fits round the selection's box plus the margin. Texts scale their height.
  Arcs stay arcs. Width and height can also be given; one of them sets the scale and the
  other follows.
- **After insert:**
  - The inserted shapes are selected. One undo step.
  - Operations aren't touched. A new shape on a layer that an operation cuts by layer is
    cut by it, as with any drawn shape.

## 5. The window (decision 8)

- ***Draw > Frame…*:** a small dialog.
  - Fields: the frame as a row of pictures, *r*, borders, gap, margin (when something is
    selected) or width and height (when nothing is), and layer.
  - A live preview on the canvas.
  - *Add* and *Cancel*.
- **Right-click on a selection > *Save to library…*:** the name and *Save*.
- **Library > Shapes:** a grid of pictures with names, a search field, and per entry
  *Insert*, *Rename*, *Delete* and *Show folder*. Insert has *Fit to selection* when
  something is selected.
- Every text is a translation id in the six languages.

## 6. Engraving the result

Nothing new is needed to cut a sign:
- a V-carve operation on the text;
- a follow path on the border lines with a V-bit (a V-carve of a closed border would carve
  the whole area inside it);
- a profile outside on the frame outline.

The guide's new page, *Signs and frames*, walks through this from an empty project. It adds
no feature.

## 7. The command line

```
chipload frame kinds                                         # the built-in frames
chipload frame add part.chipload --kind rounded --around 4 --margin 10 --borders 1 --gap 5
chipload frame add part.chipload --kind oval --width 300 --height 150 --borders 2
chipload shape save part.chipload 4 7 9 --name "House number"   # (also --texts IDS)
chipload shape library                                        # the entries, with errors
chipload shape insert part.chipload "House number" --at 150,80 # --around IDS --margin, --width, --layer
chipload shape library rename "House number" "Plate"         # also: remove, path
```

`--json` on each, through `out::Out`, as every command. The MCP server gets them as tools.

## 8. Where the code goes

| Layer | What |
|---|---|
| `chipload-draw` | `frames`: each frame's outline from width, height and *r*; the borders by offset; the fit round a box (§3.3). Geometry only. |
| `chipload-job` | the entry format (`ShapeEntry`, `SHAPES_VERSION`, migrations, `load_shape_entry` and `save_shape_entry`); the settings folder's `shapes_path`. |
| `chipload-pipeline` | `frames::add_frame(job, …)`; `shape_library::{list, save, insert}`: ids, scaling, texts and letters, DXF and SVG through the importer. |
| `chipload-cli` | `frame`, and `shape save`, `insert` and `library`. |
| `chipload-gui` | the dialog, the menu entries, the Shapes tab. No geometry. |

The browser build has frames and inserting from memory. It has no `shapes` folder (as for
fonts).

## 9. Tests

- **Each frame:** closed; doesn't cross itself; its box is width × height; *r* at zero and
  at its largest; *r* too large refused.
- **Borders:** each at exactly the gap from the one outside it (`nearest_distance`), on the
  inside; 3 borders on a small frame refused when they would vanish.
- **Round a box:** the box plus margin lies inside the innermost line for every frame, for
  wide, tall and square boxes (proptest).
- **Entries:**
  - save → insert gives the same shapes moved, with new ids;
  - texts stay texts and stay editable; a text in a missing font is locked;
  - a too-new file is refused; a DXF without units is refused on insert;
  - the frozen sample `tests/corpus/shapes/v1/` loads (the format's first version).
- **Golden:** one project, a text framed with *Rounded* and one border, cut with a V-carve
  and a profile: `v35_sign_frame`.
- **Window:** the dialog adds one undo step; the Shapes tab lists, inserts and renames; a
  screenshot of each for the guide.

## 10. Decisions

1. **Two features in one spec, built in two steps:** frames first (§3), then the library
   (§4). *Recommended:* frames alone already shorten the sign workflow, and the library
   reuses the clipboard's shape JSON. Alternative: the library only, with the frames
   shipped as built-in entries. But fixed shapes scaled to a new size stretch their
   corners: an 8 mm corner radius on a 200 mm plate becomes 16 mm on a 400 mm one, and
   uneven on a long one.
2. **The seven frames in §3.1** (rectangle, rounded, notched, chamfered, oval, arched,
   plaque), each from width, height and one *r*. *Recommended:* they cover most signs and
   stay simple. Alternatives: fewer; or more, such as banners, ribbons and shields, later
   on request.
3. **A frame becomes plain shapes; nothing about the frame is stored.** *Recommended:* no
   format change, and the shapes can be edited, exploded and exported like any other.
   Alternative: a "frame" object that stays live, so that changing the text resizes the
   frame. That needs a schema bump and is what the paused designs spec
   (docs/specs/designs.md) is for. It could come later through it.
4. **An entry holds shapes and texts (texts stay editable), not operations.**
   *Recommended.* Alternatives: shapes only, with texts flattened to letters (simpler, but
   a saved sign template couldn't get new words); or operations too (a full sign template,
   but tools differ between projects; better through presets).
5. **Layers `frame` and `border` by default**, changeable in the dialog. *Recommended:* the
   outline and the border lines are cut differently, and operations select by layer.
   Alternative: the current layer for all, which then needs splitting by hand.
6. **One file per entry in the settings folder's `shapes/`.** *Recommended:* easy to back
   up, share, delete and sync (Dropbox, git). Alternative: one `shapes.json` with import and
   export, like the tool library. That is tidier for many small entries, but sharing one
   frame means an export.
7. **DXF and SVG files in `shapes/` are listed and inserted too.** *Recommended:* the
   user's existing drawings become a library just by copying them in, and it reuses the
   importer. Alternative: only `.chipload-shapes` files.
8. **Window: *Draw > Frame…* with a live preview, *Save to library…* on the right-click
   menu, and a Shapes tab in the Library.** *Recommended.* Alternative: a dockable shapes
   panel beside the canvas, always visible. It takes room most projects don't need.
