# ADR 0030: Styles in their own file, with operations in the project format

Date: 2026-11-03. Status: accepted (docs/specs/styles.md, decisions 3, 5, 8).

## Context

Styles are operations saved by name and used again on a selection (CamBam's machining
styles). They need a place in the settings folder and a format that survives changes to
operations for years, without a second set of migrations.

## Decision

- **`styles.json`**, a library file of its own beside `tools.json` (`schema_version` 1,
  `written_by`), read and written through the same `LibraryFile` code, exported and
  imported through `Collection`.
- **Each style's operation is stored as JSON exactly as a project stores it**, with the
  file's `operation_schema` (the project schema it was written in). When that is older,
  each operation is wrapped in a minimal project and read through the project migrations
  (`styles::upgrade_operation`); the file is then saved at the current schema with a
  `.bak`. A style that can't be read or migrated is named and left out; the others load.
  Saving never raises `operation_schema` by itself.
- **Tools by library uid** (`tools`: the id the stored operation uses, the uid and the name
  at saving), mapped to project tools when used.
- **No feeds of their own:** a style's feeds come from its tool's cutting data when used
  (ADR 0029); the stored values are used only for a tool without any.
- **No link back:** operations made from a style don't remember it (ADR 0014's reasoning:
  a project cuts as it was saved).

## Consequences

- Every operation setting, now and later, works in styles without code of its own.
- A migration step that needs more of a project than its operations would refuse a style
  instead of carrying it; that is reported by name, never silent.
