# Several drawings

A project can hold several DXFs, for example two parts cut from one sheet, besides its
[images](images.md) and the shapes you draw. Each DXF has its own place, turn and scale, and
is updated from its own file.

```
chipload dxf add part.chipload lid.dxf --name lid     # a second DXF, where its file has it
chipload dxf list part.chipload                       # id, name, file, placement, size
chipload place part.chipload --dxf 2 --by 250,0       # move, turn or scale that one
chipload update part.chipload --from lid.dxf --dxf 2  # a new version of that one
chipload dxf remove part.chipload 2                   # it and its shapes
```

- **Where it goes:** a new DXF lands where its file has it, often on top of the first.
  [Place](placement.md) it afterwards.
- **Which one:** with several DXFs, `place` and `update` need `--dxf ID`. With one, they
  work as before.
- **Name:** the file's name without its extension, unless you give `--name`. A second copy
  of the same file is named "part 2".
- **Removing** a DXF removes its shapes. The report names the operations that used them;
  they can't run until you give them geometry again.

## The same layer in two DXFs

An operation that selects a layer cuts that layer in every DXF: both parts' "HOLES". When a
layer is in more than one, the report says so: *layer "HOLES" is in lid and part; it cuts
all of them*. To cut only one part's layer, limit the selector to its DXF:

```json
"select": {"layer": "HOLES", "dxf": 2}
```

A selector's `dxf` (or `image`, for a picture's traced shapes) keeps it to that source's
shapes. Without one, it means every source, as it always did.

The [setup sheet](../checking/sheet.md) lists every DXF and image with where it was
put, when there is more than one.
