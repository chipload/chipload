# Rest machining

A large tool clears a pocket fast, but leaves material where it doesn't fit:
- inside corners rounded to its radius;
- slots and gaps narrower than the tool.

A **rest pocket** with a smaller tool cuts only that material, instead of clearing the
whole pocket again.

## Adding one

- **From the pocket:** right-click the large tool's pocket in the Browser, then *Add
  rest pocket*, and pick the smaller endmill. Command line: `chipload op add-rest
  part.chipload --op 1 --tool 2`.
- **What you get:** a new pocket right after it, with the same shapes, depths, oversize
  and direction, cut with the smaller tool, set to *Rest after* the large tool.
- **Feeds and step down** come from your tool library's values for the small tool when it
  has them. Otherwise they're copied from the large pocket: check them.

Or set it on any pocket: *Clearing > Rest after* (command line: `chipload op rest
part.chipload --op 2 --after-tool 1`). The list offers the project's endmills larger than
the pocket's own tool.

## What it cuts

- **Worked out from the tool's size:** Chipload works out what a tool of the larger tool's
  size can't reach in the pocket's shapes. It looks at the size, not at what the earlier
  pocket's toolpath did.
- **Rings:** around each piece of that material, the small tool clears a small pocket with
  rings, within its own diameter. Along the real walls it follows the walls, so the
  corners come out with the small tool's radius.
- **Pieces too small for the small tool too** are skipped, with a note giving their size.
- **Nothing left:** if the large tool reaches the whole pocket (a circle wider than it),
  the rest pocket cuts nothing and says so.

*View > Show stock* shows the result: the corners cut by the small tool.

## Rules

- **A larger endmill:** the rest tool must be an endmill, and larger than the pocket's own.
- **A warning, not a stop:** Chipload warns when no earlier, enabled pocket cuts the same
  shapes with the larger tool at least as deep. The rest pocket is still safe on its own:
  it cuts its pieces from the start depth down in passes, like any pocket. It just leaves
  the bulk of the material.
- **No dogbones:** a rest pocket can't have dogbones. Put them on the large tool's pocket.
- **Copies, not links:** the rest pocket keeps its own copy of the shapes and depths. If you
  change the large pocket later, change the rest pocket too; the warning above catches a
  mismatch in shapes or depth.
