# Spec: a project without a DXF

Status: **implemented 2026-10-10.** Approved with all decisions as recommended (§5).

User request (2026-10-10): "Start a project without importing a DXF (draw first, import DXF
geometry later)."

## 1. What exists already

- A project's drawing may be empty, and `drawing.source` is optional: drawn shapes have no
  DXF behind them. So **the format doesn't change** (still schema v7, no bump).
- *Update drawing from DXF* (`chipload update --from`) replaces only the imported shapes and
  keeps the shapes drawn in Chipload, with their ids. Into a project with no DXF yet, it
  simply adds the DXF's shapes.

What's missing is a way to start one, and a few rough edges when the DXF comes later.

## 2. Starting an empty project

- **CLI:** `chipload new -o part.chipload [--machine NAME]`. It writes a project with no
  shapes and no operations. The library's machine profile and default tool apply, as for
  `import`.
- **GUI:** *File > New project (Ctrl+N)*. If the current project has unsaved changes, it
  asks first, as *Open* does. The new project is unsaved and named `project.chipload` by
  default.
- Stock, machine and tool get the same defaults as an imported project (10 mm thick, GRBL,
  6 mm endmill, 0.5 mm overcut). The stock size is in decision 1.

## 3. Importing the DXF later

- In the File menu, the item reads *Import DXF into project…* while the project has no DXF,
  and *Update drawing from DXF…* after that. It's one command either way: drawn shapes are
  kept, and the DXF's shapes get new ids.
- A DXF without units asks for them in the GUI, as *Import DXF* does. Today an update in the
  GUI fails with an error instead; that's fixed here for both cases. The CLI already has
  `--units`.
- The stock is left as it is (decision 2). If the drawing now extends beyond the stock, the
  report says so: "The drawing (X…–…, Y…–…) extends beyond the stock (… × … mm); check the
  stock under Job settings."

## 4. Tests

- CLI: `new` writes a valid empty project; `--machine` applies a profile; a drawn shape and a
  profile on it run end to end through the checker; `update --from` then adds the DXF's
  shapes and keeps the drawn one with its id.
- GUI: New project asks about unsaved changes; an empty canvas shows (fit view with no
  shapes doesn't break); drawing on it works; importing a DXF without units asks for them;
  the menu label changes after the first import.
- Golden: unchanged.

## 5. Decisions

1. **Stock size of an empty project.** Recommended: **300 × 200 mm**, origin bottom left.
   It's visible and editable, like every default. Alternatives: the machine profile's
   travel, when it has one, which would mean the whole bed (often much bigger than a part);
   or 100 × 100 mm, today's fallback for a drawing with no extent.
2. **The stock when the DXF arrives later.** Recommended: **leave it unchanged** and note
   when the drawing doesn't fit. The user may already have set it to the real board.
   Alternative: grow it to cover the drawing automatically, but only while it is still
   untouched since *New project*, which needs remembering "untouched".
3. **CLI name.** Recommended: `chipload new`. Alternative: `chipload import` with no DXF.
