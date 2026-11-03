# Depths and oversize

## Start and end depth

Every operation cuts from its **start depth** to its **end depth**, in mm down from the
stock top, whatever the [Z zero](stock.md#stock) is.

- **Rule:** 0 ≤ start depth < end depth.
- **Passes:** the cut goes down in equal passes no deeper than the **step down**, and the
  last pass is exactly at the end depth. Start 2, end 6, step down 2 gives passes at 4 and
  6 mm.
- **Cutting through:** an end depth below the stock bottom goes into the spoilboard. That's
  allowed up to *Max depth below stock* (the setup's Stock), with a warning; deeper is refused,
  so a typo can't reach the machine bed. For 12 mm board, an end depth of 12.3 cuts
  0.3 mm into the spoilboard. An operation with **Through** ticked goes there on purpose:
  the Report gives it as a plain note ("cuts through, 0.300 mm into the spoilboard").
- **Through:** tick **Through** next to the end depth of a profile, pocket, follow path or
  holes, and the end depth is the stock thickness plus **Below stock**, which starts at
  *Max depth below stock*. It follows the thickness: make the board 18 mm and the cut ends
  at 18.3. Untick it to type an end depth again. In the project file it is the operation's
  `through` (mm below the stock, null when not ticked); `chipload set part.chipload
  operations.1.through=0.3` ticks it from the command line, and setting the `end_depth`
  unticks it.
- **Going down:** from safe Z the tool rapids to 1 mm above the stock top, then goes down
  at the plunge feed. The air between the stock top and the start depth is crossed at the
  plunge feed too: Chipload doesn't know whether another operation already cleared it.

## What a new operation starts with

A new operation is set up from what you selected, so the usual job needs few edits:

| New | Starts |
|---|---|
| Profile of outlines | outside, **through**, with tabs (4 per contour, 8 mm wide, 3 mm high, at most half the stock's thickness) |
| Profile of holes | inside, **through**, no tabs |
| Profile of an outline with its holes | **two profiles**: *Profile 1 (holes)* inside first, then *Profile 2 (outline)* outside with tabs, both through, so the holes are cut while the part is still held |
| Pocket, V-carve, follow path | the end depth (or *Through*) of the last one of its kind in the project, else 1 mm |
| Holes | through |

- **Two profiles** select their shapes one by one, also when you picked a layer or a whole
  DXF: shapes added to it later (by [updating the DXF](../drawings/update.md)) aren't cut by
  them, and the Report says so.
- **Feeds, step downs and stepover** come from the tool's [cutting
  data](../library/tools.md#cutting-data) for the setup's material. A tool without cutting
  data takes them from the nearest operation above with the same tool, else from its
  diameter.
- **Going down:** a pocket helixes and a follow path ramps in unless the tool is marked
  centre-cutting (see [Tools](tools.md)).
- **When the selection doesn't tell** (nothing selected yet, or a traced picture's or a
  text's outlines and holes together), a profile is 1 mm deep with its side to choose.
- **On the command line:** `chipload op add part.chipload profile --dxf 1` (also
  `--layer`, `--shapes`, `--group`; kinds `profile`, `pocket`, `vcarve`, `path`, `holes`)
  makes the same operations and says what it added; `--depth`, `--through` and `--tool`
  change the defaults.

## Oversize

**Oversize** is how much more than drawn is cut, in mm: the cut moves into the material
that is kept by that much. Positive removes more; negative leaves material. In the project
file the field is `clearance`. Other CAM programs call the opposite "stock to leave" or
"allowance": there, a positive value leaves material.

| Operation | Positive oversize | Negative oversize |
|---|---|---|
| Pocket | the pocket larger all round, islands smaller | the pocket smaller, islands larger |
| Profile, outside | the part smaller | the part larger: stock left for a later cut |
| Profile, inside (a hole) | the hole larger | the hole smaller |
| Profile, on the line | not allowed | not allowed |

- **How:** the tool's centre runs at its radius minus the oversize from the drawn line.
- **Limit:** an oversize of the tool radius or more is refused. Use a profile *on the
  line*, or a smaller tool.
- **Corners:** inside corners of the kept material stay as round as the tool. Outside
  corners get rounded with the oversize as radius, because the tool reaches past them.
- **Use:**
  - a little positive oversize gives a fit, such as a lid that drops into its box;
  - a negative oversize leaves stock for a finishing pass (see also the profile's
    [finishing pass](../operations/profile.md#finishing-pass)).
