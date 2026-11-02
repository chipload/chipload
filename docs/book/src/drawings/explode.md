# Exploding a DXF, picture or text, joining ends

A DXF's shapes are read-only: they change only when the DXF is
[updated](update.md). A traced picture's shapes and a text's letters are read-only too:
they change only when the picture is traced again or the text is edited. To rework them in
Chipload, **explode** the DXF, picture or text: its shapes become drawn shapes, with
handles, values you can type, copy, mirror and delete, like the ones you
[draw](shapes.md).

## Explode a DXF

Right-click the DXF in the Browser, *Explode into shapes…*, or *Explode…* in its
properties. With the whole DXF selected on the canvas, right-click there for the same.

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

## Explode a picture or a text

Right-click the picture or text in the Browser, *Explode into shapes…*; or select it on the
canvas and right-click there.

- **A picture goes from the project** with its trace settings: it is no longer shown under
  its shapes and can't be traced again. The project file gets smaller by the picture.
- **A text's words can't be edited again;** its letters stay where they are, each outline
  and each hole (the inside of an O) a shape of its own. A text in a
  [group](groups.md) is replaced there by its letters.
- **Operations keep cutting the same shapes,** as for a DXF: an operation that cut the whole
  picture or text now selects its shapes by number. The G-code stays the same, apart from
  a V-carve's depth now and then by 0.001 mm (arcs come back a hair different).
- **No join:** traced outlines and letters are closed already. *Join ends* (below) is
  there if a centre-line trace needs it.
- Ctrl+Z undoes it until the project is saved.

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
chipload image explode part.chipload                # the project's only picture (or give its id)
chipload text explode part.chipload 2               # text 2
chipload shape join part.chipload                   # every drawn shape, gap 0.01 mm
chipload shape join part.chipload 12 13 14 --gap 0.1
```

With `--json`, `shapes` lists the shapes that came of it and `removed` those joined into
others.
