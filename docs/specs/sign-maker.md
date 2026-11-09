# Spec: refined sign frames, and a sign in one step

Status: **approved 2026-11-09** (all eight decisions as recommended: "we build it and
iterate if needed"). Building on branch `signs`, in the order of decision 8: step 1, live
frames (§4), built; step 2, frames and borders (§5); step 3, *New sign…* (§3, §6).

The user (2026-11-09), after the V-carve inlay: "After that i want more refined sign frames
and a reworked workflow around that if necessary". This spec reviews how a sign is made
today (§1), then proposes what to change (§2-§7). The decisions are in §9.

Builds on docs/specs/shape-library.md (frames, built 2026-10-31), text.md, vcarve-inlay.md,
mirror-part.md, two-sided.md, and the paused designs.md.

Clean room (CLAUDE.md): the frame outlines are plain geometry worked out here (lines, arcs,
offsets). None is taken from another program's catalogue, samples or output.

## 1. Review: an engraved sign today

From an empty project to G-code, as the guide page *Signs and frames* walks it:

| Step | What the user does | Settings to get right |
|---|---|---|
| 1 | Board: stock size and thickness, or *fit to drawing* with a margin | 3-4 |
| 2 | *Draw > Text*: words, font, height, place | 4 |
| 3 | *Draw > Frame…*: kind, margin, corner, borders, gap | 5 |
| 4 | V-carve on the text's layer: tool, depth, passes, floor pass | 4-6 |
| 5 | Follow path on `border`: the V-bit again, a depth that gives the groove's width | 3 |
| 6 | Profile outside on `frame`: an end mill, through, tabs | 4 |
| 7 | Generate, look, export | |

About 25 settings and six places in the window, for the most common thing a hobby router
makes. Every one of them has a sensible answer the program could give. What goes wrong:

1. **Three operations by hand after the frame.** Each needs its layer picked, its tool and
   its depths. The border's groove width is set indirectly, through a depth.
2. **The frame doesn't follow the words.** Change the name on a door sign and the frame is
   the wrong size: delete it and draw it again (shape-library.md decision 3 stores nothing
   about it). Making the same sign for several names repeats steps 3-6 each time.
3. **Few frame shapes.** Seven kinds, each one corner style all round. Missing, among the
   most made sign blanks: a pointed (gable) top, arched top and bottom, a shield, a banner
   with forked ends, and octagons. Borders can only repeat the outline's shape.
4. **The edge.** Most wooden signs get their outer edge chamfered or rounded. Today that is
   a fourth operation set up by hand, or done with a hand router afterwards.
5. **Raised letters** (the background cut down, the letters left standing) and **inlays**
   are different operations on the same drawing, chosen and set up by hand.
6. **Hanging it.** Two screw holes, or a keyhole slot in the back, are set up by hand, the
   slot as a back setup (two-sided.md).

Points 1, 2 and 5 are the workflow; 3 and 4 the frames; 6 can wait (§8).

## 2. What a user can do (proposed)

- **New sign…** (`chipload sign add`): words, a frame, a style and the tools in one dialog,
  and Chipload draws the sign and adds every operation, with the board fitted round it. One
  undo step. Afterwards everything is ordinary shapes and operations: change any of them.
- **The frame follows its text:** a frame made round a text remembers what it goes round.
  When the words, the font or the height change, the frame is drawn again to fit, its
  borders with it, and the operations keep cutting it.
- **More frames and corner styles**, and borders of their own kind.
- **An edge:** none, a V-bit chamfer, or (later) a round-over bit.

## 3. A sign in one step (decision 1)

*New sign…* asks, with the defaults shown:

| Group | Values |
|---|---|
| Words | the text, font, height (or *fit*: as large as the frame allows) |
| Frame | kind, corner, margin (or a fixed width × height), borders and gap |
| Style | **engraved** (V-carved letters, grooved borders), **raised** (the background between the innermost border and the letters cut down flat, letters and border left standing), or **inlay** (the letters as a V-carve inlay, vcarve-inlay.md) |
| Edge | none, or a chamfer of a width with the V-bit (§6) |
| Tools | the V-bit; the end mill for cutting out (and clearing, for *raised*) |
| Board | thickness; the board fitted round the sign with a margin |

What it makes:

- **Drawing:** the text, the frame (§4) round it on `frame`, its borders on `border`.
- **Operations**, in cutting order, in a new part named after the words:
  - *engraved:* V-carve the text; follow path on each border, deep enough for a groove of
    the border width (§5); edge chamfer; profile outside the frame, through, with tabs;
  - *raised:* pocket inside the innermost border round the letters, to the relief depth;
    then as *engraved*, without the V-carve;
  - *inlay:* the pocket and plug of vcarve-inlay.md, then borders, edge and profile as
    *engraved*.
- **Depths and feeds** from the tools' library cutting data, as any new operation.

A sign is not a new kind of object (decision 2): it is the frame of §4 plus ordinary
operations. *New sign…* is a shortcut, like `op add-inlay`.

## 4. A frame that follows its text (decision 3)

Today a frame becomes plain shapes and nothing about it is stored. Proposed: a small record
of each frame, an optional list `drawing.frames` (no schema bump; see §10 for why a list and
not a field on the outline):

```json
"frames": [{ "outline": 7, "borders": [8], "kind": "rounded", "corner": 8.0, "gap": 5.0,
             "around": { "texts": [1] }, "margin": 12.0,
             "center": [150.0, 80.0], "width": 160.0, "height": 63.0 }]
```

- When what it goes round changes (the text's words, font, height or place; or shapes it
  was drawn round), the frame and its borders are drawn again, as one step with that
  change. Their shape ids stay, so operations that select them keep cutting them.
- **Editing the frame by hand** (dragging a corner, moving it alone) drops the record, with
  a note: it is plain shapes from then on, as today. *Detach* does the same on purpose.
- A frame drawn to a fixed size records its size instead and follows nothing.
- `frame set ID --margin … --corner …` changes a frame's values; the window shows them in
  the frame's properties, where *Frame…* asked for them.
- An older Chipload doesn't drop the record silently: the drawing refuses unknown fields, so
  it refuses a project with a live frame and says so (as for every optional field so far).

## 5. More frames, corner styles and borders (decision 4)

**New kinds** (each from width, height and one size *r*, like the seven):

| Frame | Outline |
|---|---|
| Gable | a rectangle with a pointed top, rising *r* above the top corners |
| Double arch | arched top and bottom, each rising *r* |
| Shield | straight top, sides going down into a point, the point *r* below the sides' ends |
| Banner | a long rectangle whose short ends are cut in as a V (forked), *r* deep |
| Octagon | the chamfered frame with *r* chosen so all eight sides are equal |

**Corner styles** for the rectangular kinds: today rounded, notched (concave quarter
circle), chamfered and sharp are four separate kinds. Proposed: one *Rectangle* kind with a
**corner style** (sharp, round, notch, chamfer, and a new **step**: a square notch) and *r*.
The existing kinds stay readable in projects and on the command line as names for these.

**Borders:**
- **Width**, not depth: a border is a groove *w* wide at the surface. Its follow path's
  depth is worked out from the V-bit, w ÷ 2 ÷ tan(α/2), and shown.
- **Their own corner style:** an inner border may have a different corner style from the
  outline (a rounded sign with a notched inner line), at the same gap.
- **Double line:** a border of two grooves close together (*w* apart), counted as one
  border.

## 6. The edge (decision 5)

**Chamfer:** a follow path along the outline with the V-bit, its centre on the line, deep
enough for a chamfer of width *c* at the surface (depth c ÷ tan(α/2)), cut before the
profile so the outline is still held by the board. Half the groove falls outside the sign
and goes with the waste. The sign's profile then cuts through below it. Needs nothing new
in the toolpath. A **round-over** needs a round-over bit, which Chipload's tools don't
describe yet (§8).

## 7. Command line, window, sheet

```
chipload sign add sign.chipload "THE NÄSLUNDS" --font "Roboto Slab Bold" --height 30 \
    --frame rounded --corner 8 --borders 1 --border-width 2 --style engraved \
    --edge chamfer:3 --vbit 3 --end-mill 1 --thickness 18
chipload frame set sign.chipload 12 --margin 15 --corner 10     # a live frame's values
chipload frame detach sign.chipload 12                          # plain shapes from now on
chipload frame kinds                                            # with the corner styles
```

- **Window:** *New sign…* in the ribbon's DRAW group beside *Frame*, and on an empty
  project's start page. The dialog shows the sign as it will look (frame, text, borders)
  and a one-line list of what it will add. The frame's properties panel shows its values
  while it is live.
- **Sheet:** nothing new; each operation is ordinary.
- **Guide:** *Signs and frames* starts with *New sign…*, then explains the parts.

## 8. Later, not now

- Text along an arc (for arched and double-arch frames): a text feature of its own.
- A list of names, one sign each (name plates): with copies of a part, each its own text.
- Keyhole slots and hangers in the back: a back setup (two-sided.md) and a T-slot bit,
  which the tool model doesn't describe.
- Round-over edges: a round-over bit in the tool model.
- Ornamental corners (scrolls, fleurons): in the shape library, as user entries.

## 9. Decisions

1. **New sign… makes a whole sign in one step** (§3), drawing and operations, in a part of
   its own. *Recommended:* it removes about 20 of the 25 settings of §1 for the common
   case, and is a shortcut over what exists, as `op add-inlay` is. Alternative: keep the
   steps, and only add *Add operations for this sign* after *Frame…*.
2. **A sign is not an object**, only its frame follows its text (§4). *Recommended:* the
   operations stay ordinary and editable. Alternative: a sign as the first *design*
   (designs.md, paused): everything drawn again from its values, operations included. More
   powerful for many similar signs, but it needs the designs framework and a schema bump,
   and editing one operation by hand would fight the design.
3. **The frame follows its text** through an optional `frame` record on its outline (§4);
   editing the frame by hand detaches it. *Recommended:* fixes the most common annoyance
   (changing the words) with no schema bump. Alternative: keep frames plain, and add *Fit
   frame again* that redraws a selected frame round the selection with the dialog's last
   values.
4. **Frames:** five new kinds (§5), corner styles in one *Rectangle* kind with a new *step*
   corner, borders set by groove width, borders with their own corner style, and double
   lines. *Recommended* all; each is small. Pick fewer if you like: the new kinds and the
   border width matter most.
5. **The edge:** a chamfer with the V-bit (§6) now; round-over later with a new tool type.
   *Recommended.*
6. **Styles:** *engraved*, *raised* and *inlay* in the first version. *Recommended:* raised
   is one pocket more, inlay reuses `add_inlay`. Alternative: engraved only at first.
7. **Raised style's relief depth** default 3 mm, the background cleared with the end mill
   and the letters' inside corners left at its radius (no V-bit clean-up in the first
   version). *Recommended.* Alternative: a V-bit clean-up of the corners round the letters,
   as the V-carve floor pass does, which is more work.
8. **Order:** live frames (§4) first, as they help the frames that exist; then the new
   frames and borders (§5); then *New sign…* (§3) with the edge (§6). *Recommended.*
   Alternative: *New sign…* first, for the quickest visible change.

## 10. Settled while building

Step 1 (live frames), 2026-11-09:
- **A list, not a field on the shape.** The records are `drawing.frames`, beside
  `drawing.texts`, each naming its outline and borders by shape id. Shapes are written flat
  (`id`, `layer`, `kind`, the kind's fields); a nested record there would be the only one.
  A frame is named by its outline's id everywhere (`frame set ID` takes a border's too).
- **What it goes round** is kept by text and picture id (`around.texts`, `around.images`),
  since their letters and traced shapes get new ids when they are made again, and by shape
  id for other shapes.
- **One pure function follows every frame**, `follow_frames(job)`, after any edit: the
  window calls it when it records an undo step, so the frame changes in the same step; the
  command line in the one function every command saves through (and after `--set`). It
  compares each frame's shapes with what its record says it last drew (`center`, `width`,
  `height`): the same, it draws the frame where it now goes; already there (moved with what
  it goes round), it keeps following; otherwise it was edited by hand and is let go of.
  A frame whose shapes are deleted, or whose texts and shapes are all gone, is let go of
  too. Each time a note says so.
- **Validation** checks a record's numbers and that no shape is in two frames. It doesn't
  refuse a record naming shapes or texts that are gone: the edit that removed them is
  checked before frames are followed, and following lets such a frame go.
- **A turned frame** isn't followed: turning a frame with its text gives a turned outline,
  which isn't what the record would draw, so the frame is let go of. Turning frames with
  their text can come later if wanted.
