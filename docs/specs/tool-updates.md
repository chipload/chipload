# Spec: keeping project tools up to date with the library

Status: draft (2026-11-03), for review.

## 1. Why

The user, 2026-11-03: "when changing the tool in the library there should be an option to
update in current project. Also when opening old projects where the project tool
missmatch we should get a question if we would like to update from library. At least i
think so, prove me wrong." Agreed the same day: an offer after saving a library tool, and
on opening a line in the Report instead of a question.

Today a project tool remembers the library tool it came from (library-in-browser.md §2).
Its properties say *differs from your library: rpm 18000 -> 16000* with **Update from
library**, and its Browser row is marked. Nothing points at it unless you open the tool.

Why no question when opening (decided 2026-11-03):
- **The copy is on purpose** (ADR 0014): a project carries its tools so it cuts as it did.
  Opening a proven job to cut it again should not offer to change it by default; a habit of
  clicking *Yes* would one day change one without a look.
- **Projects from another computer** were made with another library: the question would
  come every time, and *No* would be the right answer every time.
- **Speeds from materials** (cutting-data.md §6): a setup's material sets its tools' speed
  from their rows, so those tools differ from the library by design.

## 2. After saving a library tool

When **Save** writes a library tool and the open project has tools linked to it
(`library_uid`) whose values now differ, a line under the buttons says so, one per such tool:

> T3 in this project differs: diameter 6 mm -> 6.35 mm. **Update in this project**

The button gives the project tool the library tool's values, as **Update from library** on
the project tool does, as one undo step. Nothing changes without the click. The line goes
when the tool is updated, the form is left, or the project closes.

## 3. When a project opens

The Report, after its first lines, gets one line per project tool that differs from its
library tool, with what differs, and a line with **Update all from library**:

> T3 "6 mm 2F": your library's tool has other values: diameter 6 mm -> 6.35 mm.
> Update all from library

- Clicking a tool's line opens that tool (as a line about an operation opens it).
- **Update all from library** updates every differing tool, as one undo step, and says
  which in the Report.
- Tools not from the library, or whose library tool is gone, get no line: there is nothing
  to update from (their properties say so, as today).
- The same button is in the right-click menu of the project's **Tools** folder, and the
  Tools page (*Tools* in the Browser) has it beside its list, for later.

## 4. Speeds from the material don't count

A project tool's speed differs from its library tool's when the setup's material set it from
the tool's row (cutting-data.md §6). That is not something to update:
- **Comparing:** a speed equal to the row's speed for the tool's setup's material (the
  project's material, else none) is not listed as a difference, in the properties, the
  Browser mark, §2 or §3.
- **Updating** keeps that speed: the library tool's other values, the row's speed.

## 5. On the command line

- `chipload tool compare part.chipload` (as today) leaves out speeds that come from the
  material, as §4.
- `chipload tool update part.chipload [--tool ID]`: the project's tools linked to the
  library take their library tools' values (all, or tool ID), as §3; prints what changed
  per tool; `--json`.

## 6. Decisions to review

1. **After Save, an offer per differing project tool** (§2), never automatic.
   *Recommended:* the change you just made is where you'd want to apply it.
2. **No question when opening; Report lines and Update all** (§3). *Agreed 2026-11-03.*
3. **Report lines can open a tool**, as they open an operation today. *Recommended:* the
   same is wanted for a DXF's shape (TODO: drawing warnings that select their shape), so a
   line gets a target: an operation or a tool now, a shape later.
4. **A speed from the material is not a difference, and updating keeps it** (§4).
   *Recommended:* otherwise every project with a material would differ.
5. **`chipload tool update`** (§5). *Recommended:* the window's actions on the command
   line first (architecture rule 5).

## 7. Tests

- Saving a library tool whose project tool differs shows the line; the button updates it in
  one undo step; a tool that doesn't differ gets no line.
- Opening a project with a differing tool lists it in the Report, its line opens the tool,
  *Update all* updates every differing tool in one undo step; a tool not from the library,
  or missing there, gets no line.
- A project with a material whose row sets a speed: that speed is not a difference; updating
  keeps it.
- `tool update` (all, `--tool`), `tool compare` leaving out the material's speed; `--json`.
- The Report line and the Tools folder's menu in every language (layout tests).
