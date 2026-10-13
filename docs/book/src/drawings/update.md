# Updating from a new DXF

When the drawing changes, *File > Update drawing from DXF…*, or `chipload update
part.chipload --from part.dxf`, takes in the new version and keeps the operations.

- **Same shape, same id:** shapes are matched by their DXF handle, the DXF's own id for each
  entity, which CAD programs keep when you edit. An operation that selected a shape keeps it.
- **Placed as before:** the new drawing gets the same move, turn and scale.
- **Drawn shapes stay:** shapes drawn in Chipload are kept, with their ids.
- **The report** lists what's unchanged, changed, added and removed, and what that means
  for each operation:
  - *geometry changed; check its toolpath*;
  - *layer X now uses the updated drawing*;
  - *shapes … no longer exist; choose its geometry again*: the operation can't run until
    you give it geometry again. The command line exits with 1 in that case.
- **Units:** taken from the last import. `--units` overrides them, if the new DXF is
  different.
- **Stock:** the stock isn't changed. If the drawing now reaches past it, the report says
  so.

An empty project (*File > New project*, `chipload new`) takes its first DXF the same way:
*File > Import DXF into project…*.

A DXF saved by a program that gives every entity a new handle on each save makes every
shape look new. Operations that select by **layer** survive that; ones that select single
shapes need their geometry chosen again.
