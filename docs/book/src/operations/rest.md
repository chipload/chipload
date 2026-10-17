# Rest machining

A large tool clears a pocket fast, but leaves material where it doesn't fit:
- inside corners rounded to its radius;
- slots and gaps narrower than the tool.

A **rest pass** with a smaller tool cuts only that material, instead of clearing the whole
pocket again. (Design: `docs/specs/rest-passes.md`.)

## Adding one

- **Right-click the pocket** in the Browser, then *Add rest pass ▸*, and pick a smaller
  endmill: the project's tools come first, then those in your tool library. A library tool
  is copied into the project's Tools. When the entry is greyed out, point at it to see
  why: no smaller endmill in the project or your library, or the pocket has dogbone
  corners. Command line: `chipload op add-rest part.chipload --op 1 --tool 2` (copy a
  library tool in first with `chipload tool use`).
- **It belongs to the pocket:** it shows as a leaf under it, numbered after it (*1.1*, *1.2*),
  and takes the pocket's shapes, depths, oversize and direction. Change the pocket and the
  pass follows; turn the pocket off and its passes aren't cut either.
- **Its own:** the tool, feed, plunge feed, step down, stepover, entry, and on/off. Click
  the pass to change them; its right-click menu removes it.
- **Feeds and step down** come from your tool library's values for the small tool when it
  has them. Otherwise they're copied from the pocket: check them.
- **Several passes:** a pocket can have more than one, each with a smaller tool than the one
  before (12, then 6, then 3 mm). Each clears what the one before it left.

Command line: `chipload op rest-pass part.chipload --op 1` lists a pocket's passes, and with
`--pass 1` changes one (`--tool`, `--feed`, `--plunge-feed`, `--step-down`, `--stepover`,
`--enabled false`) or removes it (`--remove`).

## When it is cut

A rest pass waits for the **pockets next to its pocket that use the same tool**. After the
last of them come all their rest passes, the larger tools first, then the operations after
them. So with two pockets and an outline, all cut with a 12 mm tool, and a 4 mm rest pass on
each pocket:

| Cut | Tool |
|---|---|
| Pocket 1, Pocket 2 | 12 mm |
| Rest passes 1.1, 2.1 | 4 mm |
| Outline 3 | 12 mm |

The small tool comes in once, and the outline, which frees the part, stays last. The report,
the G-code headers and the setup sheet show the passes where they are cut.

## What it cuts

- **Worked out from the tool's size:** Chipload works out what a tool of the larger tool's
  size can't reach in the pocket's shapes. It looks at the size, not at what the pocket's
  toolpath did.
- **Rings:** around each piece of that material, the small tool clears a small pocket with
  rings, within its own diameter. Along the real walls it follows the walls, so the
  corners come out with the small tool's radius.
- **Pieces too small for the small tool too** are skipped, with a note giving their size.
- **Nothing left:** if the large tool reaches the whole pocket (a circle wider than it),
  the pass cuts nothing and says so.

*View > Show stock* shows the result: the corners cut by the small tool.

## Rules

- **A smaller endmill:** a pass's tool must be an endmill smaller than the tool before it.
- **No dogbones:** a pocket with dogbones can't have rest passes; its corners need no rest.

## A rest pocket on its own

When the large pocket is cut elsewhere, in another project or another program, a pocket can
rest after a tool by itself: *Clearing > Rest after* in its properties (command line:
`chipload op rest part.chipload --op 2 --after-tool 1`). It is an ordinary operation in the
list, cut where it stands. Chipload warns when no earlier, enabled pocket cuts the same
shapes with that tool at least as deep; it is still safe, it just leaves the bulk of the
material.

A rest pocket that copies an earlier pocket of the project (same shapes, depths, oversize
and direction, after its tool), as *Add rest pocket* made them before rest passes, can
become that pocket's rest pass: right-click it, *Make it a rest pass of operation N*
(command line: `chipload op make-rest-pass part.chipload --op 2`). It is then cut after the
run of pockets it belongs to.
