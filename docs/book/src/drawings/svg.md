# Importing an SVG

Logos and artwork often come as SVG, from Inkscape, Illustrator, Affinity and the like. An
SVG is a drawing file just like a DXF: imported, placed, turned and scaled, updated from a
newer file, removed or exploded the same way, with the same commands.

```
chipload inspect logo.svg                          # what Chipload sees in it
chipload import logo.svg -o logo.chipload          # a new project from it
chipload dxf add part.chipload logo.svg            # beside the project's DXFs
chipload update part.chipload --from logo.svg --dxf 2   # a new version of it
```

In the window: *File > Import DXF or SVG…* (Ctrl+I) or *File > Open…* starts a project
from it, *Add DXF or SVG to project…* adds it, and dropping an SVG on the window does either,
as for a DXF. The file dialogs offer both kinds.

The SVG shares the DXFs' numbers: `--dxf 2`, `place --dxf 2` and `dxf remove … 2` work on
it, and `dxf list` shows its format. Compressed `.svgz` files are read too.

## What is read

- **Every path,** and every rectangle, circle, ellipse, line, polyline and polygon, with
  their transforms, styles and `use` copies. **Each subpath is a shape:** closed when it
  ends with Z or where it started, open otherwise.
- **The path itself, not its look:** a shape follows the path's centre line, what a laser
  or a router follows. Fill, stroke width and colour don't change the geometry. A path you
  can't see in the file (no fill and no stroke, or hidden) is still read, and the report
  says so.
- **Curves** become lines and arcs within 0.01 mm, as [text](text.md) does, so circles and
  round corners are cut as arcs (G2/G3).
- **Holes:** a shape inside another is a hole, by where it lies, as in a DXF; the file's
  fill rule doesn't matter.

## Layers

- **Inkscape layers** are the shapes' layers: a path in the layer *Cut* is on layer *Cut*
  (in nested layers, the outermost one). A path outside every layer is on layer `0`.
- **Without layers,** shapes go on their **stroke colour**, written as `#ff0000` (the fill
  colour when there is no stroke): the "red cuts, blue engraves" habit of laser work. Select
  a colour's layer to cut it.

## Size

- **Most files give their real size** (`width="210mm"`, inches, points): Chipload reads it
  exactly, and the page's **lower-left corner** becomes the drawing's origin, so the drawing
  sits as it does in the editor.
- **A file in plain pixels** gives no real size: Chipload asks how many pixels make an inch,
  as it asks a DXF without units. Inkscape and browsers use **96**; older Illustrator files
  use **72**; any other number can be typed. On the command line: `--px-per-inch 96`. The answer is kept with the file and
  used again on update.

## Left out, and said so

The import report (`inspect`, and under the canvas) names what wasn't read:

| Issue | Meaning |
|---|---|
| `svg_text` | text isn't read: it would need the fonts of your computer, and could cut differently on another one. Turn it into paths in the editor (Inkscape: *Path > Object to Path*; Illustrator: *Create Outlines*), or use Chipload's own [text](text.md) |
| `svg_image` | pictures inside the SVG aren't read; [trace a picture](images.md) on its own |
| `svg_clip` | clip paths and masks are ignored: what they clip is read whole |
| `svg_hidden` | paths that don't show in the file were read |
| `svg_empty` | subpaths with no length were left out |
| `svg_px_chosen` | the file gave no real size; this records the pixels per inch chosen |

Links to other files or to the web are never followed.
