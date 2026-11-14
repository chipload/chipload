# The new project dialog

Status: **approved 2026-11-14** (the user: "1 page for sure. all settings including,
machine, material, thickness and tools should remember its values from the previous
started project. Often it is the same next time. the rest sounds good").

## 1. Why

A new project starts from the library's defaults: the default machine, material and tool,
and one default thickness (10 mm if never set). Then the Stock form opens and the report
says to check the board. It is easy to start drawing or adding operations at once, on the
wrong board, with the wrong tools. The board's thickness decides every through cut, and Z
zero decides where the machine cuts.

## 2. The dialog

One page, before the project exists. It opens for **New project** (Ctrl+N, File menu,
ribbon), **Import** of a DXF or SVG as a new project, and a **dropped drawing file**:

```
 New project ─────────────────────────────────────────────────────────────
  Board                                   Tools                 [search  ]
  Material   [ Birch plywood    ▾ ]       ☑ T1  6 mm end mill
  Thickness  [ 18    ] mm                 ☑ T2  3.175 mm end mill
  Board      ( ) Fit the drawing, [10] mm ☐     6 mm end mill R1
             (•) Size [600] × [400] mm    ☑ T3  V 60°, 12.7 mm
  Z zero     (•) Top  ( ) Spoilboard      ☐     5 mm drill
  Machine    [ Shapeoko 4       ▾ ]
             GRBL · mm · one file per tool
                                          3 tools: T1, T2, T3
                                                 [ Cancel ]  [ Create ]
```

- **Material:** the library's materials and *None*. Choosing one with a thickness of its
  own (§3) fills in the thickness, which can still be changed.
- **Thickness:** the board's, in the display units.
- **Board:** *Fit the drawing* with a margin (the stock centred on the drawing, following it:
  `origin: drawing`, `fit`), or *Size* X × Y. While the drawing is empty, a fitted board has
  the size's X × Y.
- **Z zero:** the top of the board or the spoilboard (`z_zero` top or bottom).
- **Machine:** every profile in the library, always shown, with a line saying what it
  brings (post, G-code units, tool changes).
- **Tools:** every tool in the library, with a search over name, type and diameter. The
  ticked ones are copied into the project, linked to their entries. Each gets its library
  number, and a number already taken by a tool ticked above gets the next free one. The T
  column shows the numbers they will get.
  - None ticked is allowed: the report says to add a tool.
  - With an empty library, the column says so and offers *Add tools in the library…*, and
    the project keeps Chipload's own 6 mm end mill.
- **Enter** creates, **Escape** cancels. *Cancel* makes nothing, and the open project stays.
- There is no setting to skip the dialog: Enter accepts it.

## 3. Materials with a thickness

A library material can have its usual thickness: *Birch plywood* 18 mm, *MDF* 12 mm.
`materials.json` gains an optional `thickness` per material, left out when none, so the
schema version stays. A Chipload older than this refuses a materials file that uses it, as
0.32 did with `default_thickness`. Materials had a `thickness` until schema 2, dropped
because choosing a material never changes a project's stock (the user, 2026-10-28). That
still holds for `material use`: the thickness here is only what the new project dialog
offers. Command line: `chipload material set NAME --thickness 18`
(`--thickness 0` clears it). In the window, a *Thickness* field in the material's row.

## 4. Remembered from the last project

Every choice is kept from the last project started in the window, since the next one is
often the same: material, thickness, board (fit or size, margin, X × Y), Z zero, machine
and tools. They live in the app settings as an optional `new_project` record, left out
until the first project. Settings stay version 3; a Chipload older than this starts with
default settings once this one has saved them, as before for `reverse_orbit`. Machine,
material and tools are kept by library uid. One that has left the library falls back to
the library's default, and a missing tool is simply not ticked.

Without a record (the first time), the library's defaults:

- the default machine and material;
- the material's thickness, else `default_thickness`, else 10 mm;
- the board fitted round the drawing with a 10 mm margin (300 × 200 mm while the drawing
  is empty), as new projects have been since 2026-10-28;
- Z zero on top;
- the default tool.

## 5. Command line

`chipload new` and `chipload import` take the same choices, and without them keep the
library's defaults as today, so scripts don't change. The command line doesn't remember;
that is the window's convenience.

- `--material NAME` (or `--no-material`), `--thickness 18`, `--size 600x400` or `--fit 10`,
  `--z-zero top|spoilboard`, `--machine NAME` (already there), `--tools "6 mm end
  mill,V 60"` (library tool names, comma-separated).
- One pipeline function does it for both (`library::start_project`), and the window calls
  the same one.

## 6. Afterwards

The project opens on its Stock, as since 2026-11-03 (the user: "When starting a new
project we should open the stock setup first"). The report's first lines say what was
chosen: the material and thickness, the board, Z zero, the machine and the tools.

## 7. Tests

- Pipeline: each choice applied; tool numbers with a clash; a missing uid falls back; no
  tools; a material's thickness filling in; fit vs size; the defaults without a record.
- Command line: each option; unknown names refused with the library's names listed;
  without options the project is as before (the existing tests).
- Settings and materials: round trips, left out when none.
- Window: Ctrl+N opens the dialog, Enter creates with the remembered choices, Cancel
  keeps the open project, Import and drop go through it, the search, the record written
  on Create; layout in every language and in a narrow window.

## 8. Decisions (as approved)

1. One page.
2. Every choice remembered from the last project started in the window.
3. Tools numbered from the library, clashes moved to the next free number.
4. Materials with a thickness of their own.
5. Z zero asked here.
6. No setting to skip the dialog.
7. New sign… and Relief… keep their own board questions (unchanged).
