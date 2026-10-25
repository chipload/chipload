# Exploding a DXF, joining ends

A DXF's shapes are read-only: they change only when the DXF is
[updated](update.md). To rework them in Chipload, **explode** the DXF: its shapes become
drawn shapes, with handles, values you can type, copy, mirror and delete, like the ones you
[draw](shapes.md).

## Explode

Right-click the DXF in the Browser, *Explode into shapes…*, or *Explode…* in its
properties.

- **The project forgets the DXF file.** It leaves the Browser, and *Update from DXF* is
  no longer possible for these shapes. Ctrl+Z undoes it until the project is saved;
  after that, the only way back is adding the DXF again.
- **Circles stay circles;** everything else becomes a polyline of straight and arc
  segments. An arc of more than half a circle becomes two segments.
- **Operations keep cutting the same shapes,** with the same G-code. The shapes keep their
  ids and layers. An operation that selected a layer *of that DXF* now selects its shapes
  by number, so a layer of the same name elsewhere doesn't join in; the report lists these.
- **Circles that the placement turned** start at their right-hand point afterwards, as
  every drawn circle does, so the cut may start elsewhere on them. The report names them.

Tick *Join ends closer than* to join the exploded lines straight away (below).

## Join ends

A drawing made of separate lines and arcs whose ends almost meet can't be pocketed: each
piece is an open contour. *Draw > Join ends…* joins open drawn polylines whose ends lie
closer than the gap into one, and closes a chain that comes back to its start.

- **The selected shapes,** or every drawn shape when none is selected.
- **Gap:** 0.01 mm unless you type another, at most 1 mm. Both ends move to the point
  between them; a shape moves by at most half the gap.
- **Same layer only.** Pieces on different layers stay apart.
- **Only clear pairs.** Where three or more ends meet within the gap, nothing joins there:
  which way the outline goes would be a guess. The report gives the point.
- **The joined shape keeps the lowest id** of its pieces; operations that selected any of
  them select it.
- **The report** says how many shapes became how many, the widest gap closed, and the ends
  still open with the distance to the nearest other end: the gap that would close them.

## On the command line

```sh
chipload dxf explode part.chipload                  # the project's only DXF
chipload dxf explode part.chipload 2 --join 0.05mm  # DXF 2, then join ends closer than 0.05 mm
chipload shape join part.chipload                   # every drawn shape, gap 0.01 mm
chipload shape join part.chipload 12 13 14 --gap 0.1
```

With `--json`, `shapes` lists the shapes that came of it and `removed` those joined into
others.
