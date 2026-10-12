# Spec: import and export of the tool library and machine profiles

Status: approved (2026-10-12). Builds on docs/specs/tool-library-and-machines.md
(ADR 0014). User request 2026-10-11: moving to a new computer, backups.

## 1. The idea in one paragraph

**Export** writes the library's tools (or machine profiles) to a file the user chooses.
**Import** reads such a file and adds its entries to the library. The file has the same
format as `tools.json` (or `machines.json`), so an exported file, a backup of the library
file or the library file of another computer can all be imported. Nothing about projects
changes: they keep their own copies (ADR 0014).

## 2. Files

- No new persisted format. An export is a `tools.json` or `machines.json` file: current
  `schema_version`, `written_by`, and the default entry if it is exported.
- Import reads it with the library loader: an older version is migrated in memory (tools 1
  → 2, machines 1 → 2), a newer, missing or malformed version and a file that breaks the
  rules (two entries with the same name, a diameter of 0, …) are refused with the loader's
  message. A missing file is refused (the library loader would read it as empty).
- A tools file given to the machine import (or the other way round) is refused by the
  format's own fields (`deny_unknown_fields`), with a hint which import to use.

## 3. Export

- All entries, or the entries named (`--name`, repeatable, ignoring case; an unknown name is
  refused). The default is written only when it is among them.
- The file is written atomically (as the library). An existing file is overwritten: in the
  GUI the save dialog asks first; on the command line the target is named explicitly.

## 4. Import

Entries are matched by name, ignoring case (the library's rule).

- **New name:** added at the end, in the file's order.
- **Same name, same values:** nothing to do; counted as "already in the library".
- **Same name, different values:** kept as in the library by default and listed with what
  differs; with **replace** the imported values overwrite them (the library entry keeps its
  position).
- **Default:** the library's default stays. Only a library without a default takes the
  file's default (restoring a backup into a new computer).
- The library is changed in one save (`update_tools` / `update_machines`: the previous file
  kept as `.bak`). A report lists what was added, replaced, kept and already there.
- Machine profiles name a post-processor by name. A name this Chipload doesn't know is
  imported anyway (as when typed) and fails only when a project uses it, with the usual
  message; the import report notes it.

## 5. Command line

```
chipload tool export tools-backup.json [--name "6 mm flat"]...
chipload tool import tools-backup.json [--replace]
chipload machine export machines-backup.json [--name Shapeoko]...
chipload machine import machines-backup.json [--replace]
```

## 6. GUI

The Tool library and Machines windows get **Import…** and **Export…** buttons next to "New
…". Export exports everything (a save dialog). Import opens a file dialog, then shows what
the file holds (new, already there, differing with what differs) with **Import** (keep
mine), **Import and replace** (only when something differs) and **Cancel**. The result is
shown in the window like other library messages.

## 7. Code

- `chipload_job::library`: `read_tools_file` / `read_machines_file` (missing = error),
  `export_tools` / `export_machines` (a library holding the chosen entries), and `merge`
  for both (a plan: added, same, differing; applied with keep or replace), pure and tested
  without I/O.
- CLI in `chipload-cli/src/library.rs`; GUI in `app/user_library.rs`, thin.

## 8. Decisions to review

1. **Format:** the existing `tools.json` / `machines.json` formats, one file per library; no
   combined "everything" file. *Recommended:* no new format to version, and the library
   files themselves (or their `.bak`) can be imported as they are.
2. **Conflicts:** same name with different values keeps the library's entry unless
   **replace** is chosen; never a renamed copy ("6 mm flat (2)"). *Recommended:* nothing in
   the library changes without being asked, and renamed copies pile up on every re-import.
3. **Default:** imported only into a library without one. *Recommended.*
4. **Export selection:** all, or by name on the command line; the GUI exports all.
   *Recommended:* the GUI list has no multi-select yet; all is what a backup needs.
5. **Unknown post-processor** in an imported profile: imported with a note, not refused.
   *Recommended:* the other computer may have it later; refusing blocks the whole file.

## 9. Tests

- Export then import into an empty library gives the same library (both kinds, defaults).
- Import: new, same, differing; keep and replace; positions; default rule; case-insensitive
  names; one save (`.bak`).
- Refusals: missing file, newer version, broken rules, the wrong kind of file; the library
  file unchanged after a refusal.
- An old (version 1) file imports migrated.
- CLI: export/import round trip in a temporary settings folder; `--name` unknown refused.
- GUI: the import summary and both buttons, through the app's request functions.
