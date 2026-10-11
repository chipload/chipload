# Spec: Open recent

Status: **implemented 2026-10-10.** The user asked for it ("Could we add an Open recent
menu"), and decision 10 of docs/specs/tool-library-and-machines.md placed it in the same
settings folder, as a third small versioned file. The choices below are the conventional
ones and are open to change.

## 1. Format: `settings.json` (schema version 2)

```json
{
  "schema_version": 2,
  "written_by": "chipload 0.10.0",
  "recent_projects": ["/home/ann/cnc/sign.chipload", "/home/ann/cnc/box.chipload"],
  "theme": "system"
}
```

- Most recent first, at most 10 entries, no duplicates. Paths are absolute.
- Loading and saving follow the library rules: versioned, missing file = empty, broken or
  newer file refused and never overwritten, atomic saves keeping a `.bak`.
- A path that isn't valid Unicode isn't recorded, rather than being written wrongly.
- `theme` (since version 2, decided 2026-10-11): `system`, `light` or `dark`, set under
  *View > Theme*.
- The file refuses fields it doesn't know, so every new setting is a version bump with a
  migration, like a project. Version 1 (recent projects only) migrates to 2 with `theme:
  system`, which is how it looked before; a frozen sample is in
  `tests/corpus/settings/v1/`. Chipload 0.9.0 and older refuse a version 2 file: they
  show an empty recent list, report the version when they try to record a project, and
  never overwrite the file.

## 2. Behaviour

- A project goes to the top of the list when it is opened or saved. A DXF import isn't
  recorded: it isn't a project until it is saved.
- *File > Open recent* lists the projects by file name; hovering shows the full path. *Clear
  list* empties it.
- Opening a recent project asks first if the current one has unsaved changes, as *Open*
  does.
- A project that no longer exists is reported and taken off the list.
- Failing to write `settings.json` never stops opening or saving a project. It is shown in
  the status bar.

## 3. Command line

There's nothing to open on the command line, so there's no CLI command. The list is plain
JSON in the settings folder.

## 4. Tests

- Format: round trip, order, de-duplication, the limit of 10, refused versions.
- App: opening and saving record the project, a missing project is reported and removed,
  *Clear list* empties it, there is no settings folder in tests unless a test gives one.
