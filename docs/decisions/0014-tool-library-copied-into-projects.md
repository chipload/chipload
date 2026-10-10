# 0014: Tool library and machine profiles are copied into projects

Status: accepted (2026-10-10). Spec: docs/specs/tool-library-and-machines.md.

## Context
Users want their tools and machine values (travel, safe Z, post, …) kept between projects
instead of typed into every new one. A project could either link to library entries or carry
copies of them.

## Decision
Two new persisted formats in the user's settings folder, `tools.json` and `machines.json`
(each with its own `schema_version`, `chipload_job::library`). A project **copies** what it
uses: the default machine and tool when it is created, a tool when it is picked, a profile
when it is applied. Project tools and the project machine get an optional `name` (no schema
bump) that records where the values came from; it is only used to offer "update from
library", never applied by itself. The folder is found from environment variables
(`CHIPLOAD_CONFIG_DIR` overrides), without a new dependency.

## Consequences
- A project's G-code depends only on the project and its drawing: a library change, a
  missing library or another computer can't change what a saved project cuts. `run`,
  `plan`, `check` and opening a project never read the library.
- A value fixed in the library reaches old projects only through an explicit update.
- A broken or newer library file is reported and never overwritten; Chipload falls back to
  its built-in defaults. Saves are atomic and keep the previous file as `.bak`.
- Tests run with a temporary settings folder and never touch the user's.
