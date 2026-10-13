# Spec: a whole DXF or picture, minus some shapes

Status: approved and built 2026-10-13 (the user: "go ahead"), with one change the user was told
about: a left-out shape that can't be found again blocks generating instead of being cut.

## 1. Why

User request (2026-10-13): "If I select the image all geometries should be selected for use
in the operation; I should also be able to ctrl-deselect if I want to remove some shapes."

Selecting a picture (or a DXF) selects all of it: the operation's selector is
`{"image": 1}`, which follows the picture when it is traced again. Ctrl+click on one of its
shapes today turns the selection into shapes by id. For a picture that is refused, because
traced shapes get new ids each time the picture is traced again.

## 2. Format (no version bump)

A selector gets an optional **`exclude`**: shape ids left out of a whole DXF or picture.

```json
"select": {"image": 1, "exclude": [863, 871]}
```

- **With `dxf` or `image` only:** a selector with `exclude` and neither is refused.
- **Every id** must be a shape of that DXF or picture. If not: "operations.2.select.exclude:
  shape 863 is not in image 1 any more: pick the shapes to leave out again".
- **Missing or empty** means nothing is left out. It isn't written when there's none, like
  `dxf` and `image`.
- **Older versions** refuse a file that has it (unknown field), which is the safe way: they
  never cut what was left out. Per CLAUDE.md an optional field with a safe default needs no
  version bump.

## 3. Cutting

- **A contour with any left-out shape** is not selected, whole.
- **Notes** are as for any selection.

## 4. Traced again

When a picture is traced again (new settings, size or place), each operation that leaves
out some of its shapes gets the new ids:

- **Position relative to the picture:** each old left-out shape's centre and size are
  measured against the box of all the picture's old shapes, and each new shape against the
  box of its new shapes. So a move or a new size keeps them in the same place.
- **A match:** the new shape whose relative centre is nearest, if it is within 2 % of the
  box, its relative size is within 15 %, it is open or closed as the old one was, and no
  other old shape claims it.
- **No match:** the old id stays, so the project shows the problem of §2 and generating
  waits until the shapes are picked again. Nothing left out is ever cut silently.
- **The report says so:** "Operation 2: its 2 left-out shapes found again", or "1
  left-out shape not found: pick it again".

A DXF updated from a new file keeps shape ids for shapes that stay the same. Others give
the same problem.

## 5. App

- **Ctrl+click** on a contour of the selected DXF or picture leaves it out, and Ctrl+click
  again puts it back. Ctrl+click on any other contour goes on as today (shapes by id).
- **The selection reads** "image 1, 2 contours left out". The canvas shows the left-out
  contours unselected.
- **The Browser** shows the DXF's or picture's row selected.
- **A new operation** gets `{"image": 1, "exclude": [...]}`. *Use selection* does too.

## 6. Command line

`chipload set part.chipload 'operations.1.select.exclude=[863]'` works through the
existing path setting. No new command.

## 7. Tests

- **Format:** `exclude` round trips; `exclude` without a DXF or picture is refused; an id
  not in the source is refused.
- **Plan:** a left-out contour isn't cut, and the rest is.
- **Retrace:** after a move and a new width, the left-out shapes are found again (same
  contours left out). After settings that split a shape, it is reported and blocks.
- **App:** Ctrl+click on a contour of a selected picture leaves it out and in again; the
  operation gets the right selector.
