# 3D models

A 3D model in an STL file can be carved into the board from the top as a **relief**: a
sign with raised, rounded letters, a rosette, a coin, a landscape. The model lies on the
board like a picture, seen from above, and is carved in three axes; nothing under an
overhang is reached. (Design: `docs/specs/stl-relief.md`.)

*Relief carving is being built in steps: adding and placing a model, and carving it with a
finishing pass, work from the command line. Roughing and the window's forms come next.*

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
  deep. Without it, the depth is in proportion to the width, and follows the width when it
  changes.
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

## Carving it: the Relief operation

```sh
chipload tool add "Ball 3" --type ballnose --diameter 3     # then `tool use` it in the project
chipload op add sign.chipload relief --model 1             # a relief of model 1, within its outline
chipload set sign.chipload operations.0.stepover=0.1 operations.0.angle=0.785
```

A **Relief** carves the model in parallel lines across its area, each point as low as the
tool can go without cutting into the model: the tool is lowered onto the model's triangles
until it first touches one. A ball nose gives the smoothest finish; a tapered ball nose
reaches finer detail; an end mill or a V-bit works too, with their own marks.

- **Area:** with `--model`, the model's outline: the tool's centre stays inside it, so
  round the model a groove the tool's width is left, and the board beyond is untouched.
  Clear the background with a pocket, or select other closed shapes for the area.
- **`stepover`:** the distance between lines as a share of the tool, 0.1 (10 %) by default;
  smaller is smoother and slower.
- **`angle`:** the lines' direction in radians from X (0 by default). Lines at an angle to the
  model's long edges leave fewer marks.
- **`zigzag`:** back and forth (true, the default), or every line the same way, lifting
  between them.
- **`start_depth`:** how far below the stock top the model's top lies. Its lowest point is
  the start depth plus the model's depth, which the checker holds within the stock.

In the project file a relief reads:

```json
{ "type": "relief", "name": "Relief 1", "enabled": true, "part": 1, "material": null,
  "model": 1, "select": { "model": 1, "shapes": null, "layer": null }, "tool": 2,
  "start_depth": 0.0, "end_depth": 5.0, "stepover": 0.1, "angle": 0.0, "zigzag": true,
  "feed": 1500.0, "plunge_feed": 500.0 }
```

`model` names the model it carves; `end_depth` is made again from the model's depth
whenever the project is read or changed.

Along each line the tool stays within 0.01 mm of where the model needs it: more points are
added where the surface bends. When the program is made, the path is checked against the
model once more by a simpler test of its own; if anything would cut into the model, nothing
is made and the report says where.

