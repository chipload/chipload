# Spec: tool library and machine profiles

Status: **approved 2026-10-10** (decisions 1–10 as recommended). ADR 0014.

User request (2026-10-10): save a tool library per user, and machine preferences (XYZ
travel etc.), so they aren't entered again for every project. "Give a suggestion for a
simple and robust solution."

## 1. The idea in one paragraph

The user's tools and machines live in two small versioned files in their settings folder.
A project **copies** what it uses: the machine's values when the project is created, and a
tool when an operation picks it. The project file stays complete on its own, exactly as
today. It opens, plans and runs the same on another computer, or after the library changed.
**The G-code depends only on the project and its drawing, never on the library.** The
library is a source of starting values, nothing more.

## 2. Where the files are

| OS | Folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/chipload/`, else `~/.config/chipload/` |
| Windows | `%APPDATA%\chipload\` |
| macOS | `~/Library/Application Support/chipload/` |

- `CHIPLOAD_CONFIG_DIR` overrides the folder (tests, a portable install, a shared network
  folder for a workshop).
- Found from environment variables, about 20 lines, no new dependency.
- The folder is created only when the user first saves something into it. Chipload never
  writes it on its own.
- The CLI and the GUI show the full path wherever the library is listed, so the user can find
  it and back it up.

## 3. Formats (two new persisted formats, schema version 1 each)

`tools.json`:

```json
{
  "schema_version": 1,
  "written_by": "chipload 0.6.0",
  "default_tool": "6 mm 2-flute upcut",
  "tools": [
    {"name": "6 mm 2-flute upcut", "id": 1, "type": "endmill", "diameter": 6.0,
     "flutes": 2, "center_cutting": true, "rpm": 18000.0,
     "cutting": {"feed": 1200.0, "plunge_feed": 300.0, "step_down": 2.0, "stepover": 0.4}}
  ]
}
```

- Each entry is a project tool (`id`, `type`, `diameter`, `flutes`, `center_cutting`,
  `rpm`) plus a `name` and optional `cutting` values.
- `cutting` fills feed, plunge feed, step down and stepover when an operation is *created*
  with this tool. It never changes an existing operation. There is one set per tool;
  values per material can come later.
- `default_tool` is optional: the tool a new project starts with (§5).

`machines.json`:

```json
{
  "schema_version": 1,
  "written_by": "chipload 0.6.0",
  "default_machine": "Shapeoko",
  "machines": [
    {"name": "Shapeoko", "post": "grbl", "safe_z": 5.0, "spindle": "cw",
     "max_feed": 5000.0, "travel": {"x": 830.0, "y": 830.0, "z": 95.0},
     "work_zero": {"at": "stock", "corner": "bottom_left"}, "max_overcut": 0.3}
  ]
}
```

- A profile holds a project's `machine` values plus `max_overcut`.
  - `max_overcut` sits under `stock` in projects, but it is about the spoilboard, so it
    belongs to the machine.
  - `work_zero` is the corner this machine is usually zeroed at.
- Not in a profile: stock size and thickness, which belong to each job's material.
- All lengths in mm and feeds in mm/min, as in projects.

Rules for both files:
- Names are not empty, trimmed, and unique (ignoring case).
- `default_*` must name an entry.
- Values follow the same rules as in a project; the same validation code checks both.
- Loading is one function per format. It follows the CLAUDE.md rules: migrate older
  versions, refuse missing, malformed or newer versions with a typed error. Saving writes
  every field.

## 4. Names in the project (optional fields, no schema bump)

Project tools and the project's machine get an optional `name`:

```json
"machine": {"name": "Shapeoko", "post": "grbl", ...},
"tools": [{"id": 1, "name": "6 mm 2-flute upcut", "type": "endmill", ...}]
```

- The name records where the values came from. It changes nothing in planning or G-code.
- It lets the GUI notice when the library entry with the same name now has different
  values. The GUI then shows "differs from library: Update from library / Keep". Nothing
  is ever updated without that click.
- A missing name is the safe default: older files load unchanged.

## 5. Behaviour

**New project** (`chipload import`, the GUI's Open DXF, and later a project without a
DXF):
- With a default machine: the project gets that profile's values, and the report says so:
  "machine: Shapeoko (default profile)".
- Without one: today's values (GRBL, safe Z 5 mm, …).
- Tools: the library's `default_tool` if set, else today's 6 mm endmill.
- **A user with no library sees exactly today's behaviour.**

**Using a library tool in a project:**
- The GUI's tool picker for an operation lists the project's tools first, then the
  library's ("from library").
- Picking a library tool works like this:
  - If the project already has a tool with that name and the same values, it is reused.
  - Otherwise the tool is copied in. It keeps its library `id` if that id is free in the
    project, else it gets the next free id, and the GUI says so ("added as tool 3").
  - A new operation takes the tool's `cutting` values.

**Saving into the library:**
- "Save to library…" works on a project tool or the project's machine, and asks for a
  name.
- An existing name asks before it overwrites.

**Machine of an existing project:** "Use machine profile…" copies a profile's values into
the project, as one undo step. Values that change are listed first ("safe Z 5 → 10 mm").

## 6. Robustness

- **Projects never need the library.** `run`, `plan`, `check` and opening a project don't
  read it. A broken or missing library can't stop a cut.
- **Broken or newer file:** Chipload shows the error and works with built-in defaults for
  that session. It **refuses to save** over that file, so an older Chipload never destroys
  a newer one's library.
- **Saving:**
  - each save writes `tools.json.tmp`, keeps the previous file as `tools.json.bak`, then
    renames, so a crash mid-save never leaves half a file;
  - each save reads the file again first and changes only the one entry, so two windows
    don't undo each other's unrelated edits.
- Deterministic file content (entries in the user's order, fixed number format), so the
  files diff well under version control or a shared folder.

## 7. Command line (before the GUI, as always)

```
chipload tool list                                   # the library, with its path
chipload tool add "6 mm upcut" --diameter 6 [--type endmill] [--flutes 2]
                  [--center-cutting yes|no] [--rpm 18000] [--id 1]
                  [--feed 1200 --plunge-feed 300 --step-down 2 --stepover 0.4]
chipload tool set NAME [same options] [--rename NEW]
chipload tool remove NAME
chipload tool default NAME
chipload tool use NAME part.chipload                 # copies into the project, prints its id
chipload tool save part.chipload --tool 2 --name "6 mm upcut"

chipload machine list | add NAME ... | set NAME ... | remove NAME | default NAME
chipload machine use NAME part.chipload              # copies values, lists what changed
chipload machine save part.chipload --name NAME

chipload import part.dxf -o part.chipload [--machine NAME]
```

## 8. GUI

- A **Settings** menu with *Tool library…* and *Machines…*. These windows list, add, edit
  and delete entries, set the default, and show the folder path. They reuse the project's
  tool and machine forms.
- The project's machine panel gets *Use profile…* and *Save as profile…*.
- The tools panel gets *Add from library* and *Save to library*.
- The "differs from library" note is shown as in §4.
- The GUI receives the settings folder as a parameter; app tests use a temporary folder and
  never touch the real one.

## 9. Code

- The formats, validation, migrations, load/save and the "copy into project" functions go
  in a new module in `chipload-job` (`chipload_job::library`). They share the `Tool` and
  `Machine` types and their validation.
- Finding the folder (environment variables) goes in `chipload-pipeline`.
- No new dependency.
- A short ADR (0014) records the copy-not-link decision.

## 10. Decisions to review

1. **Copy into the project, never link** (§1). Recommended: the G-code can never change
   because a library file changed. The cost is that a fixed library value must be updated
   in old projects one by one, through the "differs from library" note.
2. **Location** (§2): the OS settings folder, overridable with `CHIPLOAD_CONFIG_DIR`, found
   without a new dependency. Recommended.
3. **Two files**, `tools.json` and `machines.json`, each with its own `schema_version` (§3).
   Recommended over one combined file, since tools and machines are shared and changed
   separately. Recommended over one file per machine, since that's more files to keep
   valid for little gain.
4. **A machine profile** holds post, safe Z, spindle, max feed, travel, work zero and max
   overcut, but not stock size or thickness (§3). Recommended.
5. **Tool cutting values** (feed, plunge feed, step down, stepover): one optional set per
   tool, used only when an operation is created (§3). Recommended; per-material values
   later if needed.
6. **An optional `name`** on project tools and the project machine, without a schema bump,
   used only to offer "Update from library" (§4). Recommended.
7. **New projects** take the default machine and default tool; with no library, nothing
   changes from today (§5). Recommended.
8. **A broken or newer library** is refused and never overwritten; saves are atomic with a
   `.bak` (§6). Recommended.
9. **CLI** as `chipload tool …` and `chipload machine …` (§7). Recommended.
10. **The same folder** later holds the "Open recent" list (your item 3), as a third small
    versioned file (`settings.json`) with its own short spec. Recommended.

## 11. Tests

- Round trip of both files; every validation rule; a newer version, a missing version and
  bad JSON refused with typed errors.
- `CHIPLOAD_CONFIG_DIR` is honoured. Every CLI test runs with a temporary folder.
- Import:
  - with an empty folder, the project is byte-identical to today's;
  - with a default machine and tool, the project has their values and names.
- `tool use`: reuses a matching tool, keeps a free id, renumbers a taken one.
- `machine use` lists the changed values.
- A broken library: `import` still works with defaults and says why; saving to it is
  refused.
- Atomic save leaves the old file intact if the write fails (simulated with a read-only
  folder).
- Golden G-code unchanged.
