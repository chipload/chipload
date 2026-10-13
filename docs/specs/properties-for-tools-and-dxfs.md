# Spec: tools, DXFs and pictures in Properties

Status: approved and built 2026-10-13.

## 1. Why

User request (2026-10-13):
- "When I highlight tools in the Browser I want the tools' properties in the properties
  panel. Move them from jobs."
- "When I highlight an imported drawing in the Browser I want the options for scale and
  position in the properties panel."
- "The image import dialog should not have scale and position in it, only the conversion
  settings; scale and position should be a property visible when it is highlighted."

Today a click on a tool row opens the whole job settings, with the tools in one section
among stock and machine. A DXF's placement is a floating *Place* window, opened by a
double-click or a right-click.

## 2. Tools

- **Clicking *Tools*** (the folder row) shows **Tools** in Properties, as Job settings shows
  it today:
  - *Add from library…* and *New tool*;
  - every tool in its own section.
- **Clicking a tool** (T1, T2 …) shows **that tool** alone: its rows as today (library,
  type, diameter, flutes, spindle speed, centre cutting, angle and tip for a V-bit), *Save
  to library…*, and *Remove* when no operation uses it. The title is the tool, e.g.
  "T2 · 60° V-bit".
- **Job settings** keeps the stock, the machine and the post. It no longer lists the tools,
  and its *Drawing* line ("18 shapes: 2 DXFs, 1 image") goes too: the Browser shows both.
  User request: "At the top of the job settings there is a list of shapes and drawings
  that can be removed."
- **Problems** on a tool (e.g. no spindle speed) mark its row in the Browser with the red
  dot. Clicking the problem opens the tool.

## 3. An imported DXF

- **Clicking a DXF row** selects its shapes, as today, and shows the **DXF's properties**:
  - its name, file and units, and its size;
  - the placement rows of the Place window (turn, size W/H, scale, on the stock, move by
    X Y, Reset);
  - *Update from a DXF…* and *Remove* as buttons.
- **The Selection section** shows above it, as for any selection (browser-and-layout.md
  §4).
- **The Place window goes.** Double-click and right-click > *Place…* open the DXF's
  properties instead.
- **Layer rows** keep selecting the layer. They show the DXF's properties too, since the
  layer is part of it.

## 3b. A picture

- **Clicking a picture's row** selects its shapes, as today, and shows the **picture's
  properties**:
  - its file, the picture's size in pixels and its trace mode, as facts;
  - **Width** (the height follows, shown), **lower-left corner X Y**, and the **layer**
    its shapes are on;
  - *Trace settings…*, which opens the picture's window, and *Remove*.
- **Changes apply when the edit is finished:** a drag released, or Enter or Tab in a field.
  Each change is one undo step, and traces the picture again in the background, as *Apply*
  does now. That's needed for a size change (smoothing and the smallest piece are in mm),
  and it keeps a move exactly what `image set --at` gives.
- **The picture's window keeps only the conversion settings:** threshold, invert, blur,
  smallest piece, smoothing, trace mode and arcs, with the preview. Its title says
  "Trace logo.png".
- **A new picture** comes in where it does today, half the stock's width with its
  lower-left corner at X0 Y0. *Import* closes the window and selects the picture, so its
  size and place are right there in Properties.
- **Command line:** unchanged. `image add` and `image set` keep `--width` and `--at`.

## 4. Decisions

1. **What a click shows:** clicking a tool shows that tool alone; clicking *Tools* shows
   them all, with *Add from library…* and *New tool*. *Recommended.*
2. **Tools leave Job settings, and so does the Drawing line,** with no summary left in
   their place. *As asked.*
3. **A DXF's placement lives in Properties.** The Place window is removed, and *Place…*
   opens the properties. *Recommended.*
   - Alternative: keep the window as well.
4. **A click on a DXF does both:** it selects its shapes and shows its properties.
   *Recommended,* since both are what a click on it is for.
5. **A picture's size, place and layer live in Properties;** its window keeps the
   conversion settings and the preview (§3b). *As asked.*
6. **A size or place change applies when the edit is finished,** not on every step of a
   drag. *Recommended,* since each change traces the picture again.
7. **A new picture** starts half the stock's width at X0 Y0, as now, and its properties
   open right after *Import*. *Recommended.*

## 5. Tests

- Focus on a tool, on *Tools*, on a DXF and on a picture: the forms draw, and they edit
  the same values as before (tool diameter, placement scale, picture width and corner).
- A picture's width or corner change gives the same shapes as `image set` with those
  values.
- A placement change from the DXF's properties is one undo step, as from the window.
- A tool's problem opens the tool from the problems strip.
- No change to project files or G-code.
