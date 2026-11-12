# 3D models

A 3D model in an STL file can be carved into the board from the top as a **relief**: a
sign with raised, rounded letters, a rosette, a coin, a landscape. The model lies on the
board like a picture, seen from above, and is carved in three axes; nothing under an
overhang is reached. (Design: `docs/specs/stl-relief.md`.)

*Relief carving is being built in steps. This page covers the first: adding a model and
placing it. The **Relief** operation that carves it comes next.*

## Adding a model

```sh
chipload model add sign.chipload rose.stl --width 120 --depth 6 --at 20,20
```

- **The file:** binary or text STL. It is kept inside the project, as pictures are, so the
  project stays one file; a model over 20 MB says that the project gets as large.
- **`--width`:** its width on the board; the height follows. Without it, the model keeps its
  own size.
- **`--depth`:** how deep the relief goes, from the model's top at the stock top to its
  lowest point. Reliefs are usually flatter than the model: a 14 mm tall model carved 4 mm
  deep. Without it, the depth is in proportion to the width.
- **`--at X,Y`:** where the lower-left corner of its footprint goes; **`--rotate`** turns
  it about its centre, any angle.
- **`--up`:** which of the model's axes points up: `z` (the default), `y`, `x`, or one of
  them reversed (`-z` …). Many models are drawn with Y up.
- **`--units inch`:** STL files carry no unit. The size on import is read in mm, or in inches
  with this; since the model is placed by its width, it matters only for that first size.
- **`--layer`:** the layer for its outline; by default the file's name.

The report says how many triangles it has, any without an area that were left out, and
how many open edges its surface has: a gap in a model lets the tool fall through to the
relief's floor.

## Its outline

The model's footprint seen from above becomes a closed shape on its layer, drawn again
whenever the model is moved or changed. Select it for a profile to cut the relief out of
the board, or for a pocket that clears round it.

## Changing and removing

```sh
chipload model set sign.chipload 1 --rotate 15 --depth 4
chipload model set sign.chipload 1 --proportional     # the depth follows the width again
chipload model list sign.chipload                     # file, up axis, place and size
chipload model remove sign.chipload 1                 # the model and its outline
```

`chipload set sign.chipload drawing.models.0.width=100` works as well, and draws the outline
again.
