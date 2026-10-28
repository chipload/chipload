# Exporting the drawing

Once a part is drawn or edited in Chipload, the drawing can go back out as a DXF: to a laser
cutter's software, to a shop for a quote, or into a CAD program.

```
chipload dxf export part.chipload -o part.dxf
chipload dxf export part.chipload -o part.dxf --inch --layer Cut --layer Holes
```

In the window: *File > Export drawing as DXF…*, then the choices and the file name.

## What is written

- **Every shape the canvas shows,** where it shows it: shapes you drew, a DXF's or an SVG's
  shapes as placed, traced pictures, and texts as their letters' outlines. Hidden layers
  and groups are left out unless you ask for them (`--hidden`, or the checkbox).
- **On their layers.** A layer name with a character a DXF can't hold (`< > / \ " : ; ?
  * | =`) is written with `_` in its place, and the report says so.
- **Circles as circles, everything else as polylines** whose arcs stay true arcs (bulges),
  so round corners and holes are cut as arcs by whatever reads the file.
- **Only some of it:** `--layer NAME` (repeatable) or `--shape ID …` on the command line;
  *Only the selected shapes* in the window.
- **Not written:** toolpaths (that's the G-code), the stock, the job's copies, operations.

## Units

Millimetres, and the file says so (`$INSUNITS`), so programs that read it (Chipload too)
don't ask. *In inches* (`--inch`) writes inches and says that instead.

## The file

The classic DXF layout every CAD, laser and CAM program reads (AutoCAD R12 style), as
plain text. The same project always gives exactly the same file. Before writing, Chipload
reads the file back with its own importer: if any shape doesn't come back the same, nothing
is written and the error names the shape.
