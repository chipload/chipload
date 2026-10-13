# Images

A picture, such as a logo, a scanned sketch or lettering, can be traced into shapes and then
[V-carved](../operations/vcarve.md). Chipload turns it black and white, traces the edges
between black and white, and puts the outlines on a layer of their own.

**Black is what gets carved:** for a V-carve, black is the inside of the shapes. A light logo
on a dark background needs **invert**.

PNG, JPEG and BMP pictures work, up to 20 MB and 20 megapixels. The picture is stored in the
project, so its settings can be changed later.

## Size and place

You give the picture's **width in mm**; its height follows from the picture. Chipload
doesn't use the picture's DPI, which is often missing or wrong. Its lower-left corner goes at
X, Y (0, 0 unless you say).

## Black and white

| Setting | Default | |
|---|---|---|
| **Threshold** | auto | grey values (0 black, 255 white) below it are black, 1 to 254. *Auto* picks the value that best separates the picture's dark and light parts (Otsu's method) and says which it picked. |
| **Invert** | off | carve the light parts instead |
| **Blur** | 1 px | smooths ragged edges and scan or JPEG noise before the threshold, 0 to 10 px |
| **Smallest piece** | 0.5 mm² | black or white specks smaller than this are dropped, 0 to 100 mm² |
| **Smoothing** | 0.05 mm | how far a traced edge may be straightened away from the picture's edge, 0.01 to 1 mm |

## The traced shapes

- **Smooth edges:** the edge is traced where the blurred grey crosses the threshold, between
  pixel centres, so there are no pixel steps.
- **Always valid:** the outlines are always closed and never cross. A white part inside a
  black one is a hole, as in a pocket.
- **Picture border:** black that reaches the picture's edge is closed along it, with a note:
  usually a background that wants **invert**.
- **Too many shapes:** a picture that traces into more than 5000 shapes (a photo, noise) is
  refused. Blur it more, raise the smallest piece or change the threshold.
- **Not editable:** the shapes are made of lines, and they can't be edited in Chipload.

**Changing a setting traces the picture again** and replaces its shapes, which get new ids.
So select the image's **layer** in the operation, never the shapes one by one: Chipload refuses
an operation that selects traced shapes by id.

The drawing's [placement](placement.md) moves only the DXF's shapes, never a traced picture;
a picture has its own X, Y and width.

## On the command line

```
chipload image add part.chipload logo.png --width 120 --at 10,10 --threshold auto
chipload image set part.chipload 1 --threshold 140 --blur 2 --invert true
chipload image list part.chipload
chipload image preview part.chipload 1 -o bw.png
chipload image remove part.chipload 1
```

- **`image add`:** the layer is the file's name unless `--layer` says otherwise. It reports
  how many shapes were traced, the threshold, the specks dropped, and whether black touches
  the border.
- **`image preview`:** writes the black-and-white picture exactly as it is traced.
- **`chipload set`:** also reaches the settings (`drawing.images.0.threshold=140`). The image
  is then traced again, as with `image set`, also for `--set` on a single run.

## In the file

`drawing.images` in the [project file](../reference/project-file.md#image): `id`, `file`,
`sha256`, `data` (the picture, base64), `layer`, `x`, `y`, `width`, `threshold` (a number
or `"auto"`), `invert`, `blur`, `min_area`, `smoothing`. The traced shapes are `path`
shapes with `"image": id`.
