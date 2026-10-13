# Spec: scripting (JSON Schema, JSON output, editing by path, batch)

Status: **approved 2026-10-13** ("Go ahead"), all decisions as recommended (§8).
Built 2026-10-13. As built:
- §3: every command's output goes through `Out` (crates/chipload-cli/src/out.rs), which
  prints text or collects the JSON. `--json` is a global flag, and it replaces the old
  per-command `--json` of `inspect`, `plan` and `check`, which are spelt the same way.
- §3: `plan --json` already had its warnings under `notes`, the envelope's key with the
  same meaning.
- §3: library imports give `added`, `same`, `differing` and `replaced` (the import plan's
  own words) instead of "added, skipped, replaced".
- §3: an `update` that loses geometry fails with its warning as the error message.
- §4: `check-project`'s error names the first problem. `set` refuses a value the format
  doesn't load with "the project wouldn't load after setting PATH: …".
- §2: shapes are one flat object in the file, so the schema lists every shape field as
  optional. Loading checks each kind's own fields. The `THIRD-PARTY-LICENSES.txt` entry for
  schemars carries a file it copied from regex-syntax with its MIT notice, which is the
  attribution that notice asks for.
- §5: the recipes are in `docs/book/src/scripting.md` (user guide), linked from the README.

Guide: docs/book/src/scripting.md.

Approved plan (2026-10-13, ADR 0017), item 3. This is for power users who drive Chipload
from scripts: generate or change projects, run them, and read the results, without
screen-scraping text.

## 1. Where things stand

- Three commands print JSON already: `inspect --json`, `plan --json` and `check --json`. Each
  has its own shape. `plan --json` has its warnings under `notes`.
- Every other command prints text only, and errors go to stderr as `error: …`.
- Exit codes are 0 for done, 1 for failed (including a program the checker refuses), and 2
  for a usage error (clap).
- The project format is described only by the code (`chipload-job` models) and the specs.
  Nothing a script or an editor can check a file against.
- Changing one value in a project needs a command made for it (`op tool`, `op rest`,
  `place`, `copies`…) or editing the JSON by hand.

## 2. JSON Schema for the persisted formats

- **Formats covered:** the project file (`.chipload`), the tool library (`tools.json`) and
  the machine profiles (`machines.json`) (decision 1). Settings and recovery notes are
  Chipload's own business, so they are left out.
- **Generated from the models** with `schemars` (decision 2), so it can't drift from what
  loads:
  - the serde attributes give the schema's shape: tagged enums, defaults, and
    `deny_unknown_fields` as `additionalProperties: false`;
  - the doc comments become the fields' descriptions;
  - the schema covers the **current** version only. Older files load through their
    migrations, never through the schema.
- **What it checks:**
  - covered: types, required fields, and the allowed values of enums;
  - not covered: ranges and cross-field rules ("stepover in 0..1", "end below start",
    "clearance less than the tool radius"). Loading and the problem list still check
    these, and `chipload check-project` (§4) reports them with their paths.

  A file the schema accepts may therefore still have problems. A file it refuses won't
  load (decision 3).
- **How to get it:**
  - `chipload schema project|tools|machines` prints it;
  - the three are committed under `docs/schemas/` (`chipload-project.schema.json`, …);
  - a golden-style test keeps them equal to the generated ones (`CHIPLOAD_UPDATE_GOLDEN=1`
    to update, with the diff reviewed like any golden).
- **Editors:** VS Code can be pointed at the schema for `*.chipload` (a `json.schemas`
  setting, documented in `docs/book/src/scripting.md` (user guide)). That gives completion and red squiggles. No
  `$schema` key goes into project files (decision 4).
- **Tests:**
  - every frozen corpus project at the current version validates against the schema, and
    so does every file `save_job` writes in the tests;
  - a few broken files (a misspelt field, an unknown operation type) are refused;
  - the validator is a dev-dependency only (§7).

## 3. JSON output from every command

A global `--json` flag, accepted before or after the subcommand (`chipload --json run …` and
`chipload run … --json`). With it, the command prints **exactly one JSON object** on stdout
and nothing else, also on failure. Without it, nothing changes.

**The envelope**, the same keys on every command:

| Key | |
|---|---|
| `chipload_json` | `1`, the version of this output format (decision 5) |
| `ok` | `true` when the command did what it was asked |
| `notes` | warnings and notes, the text lines the command prints today, as strings |
| `written` | files written, absolute paths, in order (empty for read-only commands) |
| `error` | only when `ok` is false: `{ "message": "…", "problems": [ … ] }` |

- `problems` lists the job's problems, each with its path, short text and full text, as
  the GUI shows them (`{"path": "operations.1.rest", "short": "…", "text": "…"}`). A script
  can then point at the field, or fix it with `set` (§4).
- **Exit codes stay as today.** A refused check gives `ok: false` with exit 1. Usage errors
  stay clap's text with exit 2, because they happen before a command runs.

**Each command's own keys**, next to the envelope's:

- `plan`: today's keys unchanged (`program`, `groups`, `pockets`, `material_left`,
  `ring_starts`, `estimate`, `chip_load`, and the warnings in `notes`, which is the
  envelope's key with the same meaning).
  - The old `plan --json` flag is the same flag, so existing scripts keep working, and gain
    the envelope keys.
- `inspect`: today's keys unchanged. `check`: today's keys unchanged; its `ok` already means
  what the envelope's does.
- `run`:
  - `files`: each file's path, tool and operations;
  - `sheet`: the setup sheet's path, or null;
  - `estimate`, in the same form as `plan`.
- `simulate`: `numbers` (removed volume, deepest cut, …) and `image` (a path, or null).
- `sheet`: no keys of its own; its path is in `written`.
- **Commands that change a project** (`new`, `import`, `update`, `copies`, `place`, `shape
  …`, `op …`, `post use`): no keys beyond `written`, except:
  - `import`: `report`, as `inspect --json` gives it;
  - `update`: `unchanged`, `changed`, `added`, `removed` (shape ids) and `lost`
    (operation indexes);
  - `shape add` and `shape copy`: `ids`;
  - `place` without options: `placement`.
- **Library commands** (`tool …`, `machine …`, `post list`):
  - `list` prints the entries as in the file (`tools`, `machines`, `posts`), plus `path`,
    the file's location;
  - `tool use`: `id`;
  - `import`: `added`, `skipped` and `replaced` (names).
- `render`: `gcode` (the text). `licenses`: `text`. `schema`: the schema itself, without an
  envelope, since it is the document asked for.

The output is deterministic: `serde_json` without `preserve_order` keeps object keys sorted,
as the CLI's JSON is today. Numbers are as stored, not rounded for display.

## 4. Reading and changing any value: `get`, `set`, `check-project`

One generic command instead of a new one for every field (decision 6):

- `chipload get part.chipload operations.1.end_depth` prints the value as JSON: `6.0`, or an
  object or array.
- `chipload set part.chipload operations.1.end_depth=6.5 [more=…] [-o out.chipload]` sets
  one or more values.
  - **Paths** are the problem paths (§3): dot-separated keys, array indexes **from 0**, as
    in the file. Note that `--op` on the other commands counts from 1, as they print
    operations (decision 7).
  - **Values:** each is read as JSON when it parses, otherwise as a string. So `6.5`,
    `true`, `null`, `{"after_tool": 1}` and `Outline` (a string) all work. Quoting decides
    where it's ambiguous: `name='"6"'`.
  - **Adding:**
    - setting an index one past the end of an array appends, e.g. `operations.3={…}` adds a
      fourth operation;
    - a missing key is added if the format has it (an optional field such as `rest`).
  - **Removing:** `set` doesn't remove; setting an optional field to `null` turns it off.
  - **Checked as a whole before writing:**
    - the changed project must load as the current version and pass validation;
    - otherwise nothing is written, and the error names the path;
    - `schema_version` and `written_by` can't be set.
  - **Problems don't block writing:** a value the planner would refuse is written, and the
    problems are listed, as the GUI keeps an edit with a red field.
- `chipload check-project part.chipload` lists the problems (the GUI's live problems), with
  their paths, without planning. It exits 1 if there are any.
- **`--set path=value` on `run`, `plan`, `simulate` and `sheet`:** for that run only, and
  nothing is written to the project. For example, `chipload run part.chipload --set
  stock.thickness=18 -o part-18.nc`. Repeatable.

`op tool`, `op rest`, `copies`, `place` and the `shape` commands stay. They do more than set
one value: they check tools, move drawings and keep ids.

## 5. Batch runs: shell recipes, not a command (decision 8)

A loop in the shell already runs many projects, in parallel if wanted (`xargs -P`), and
`--json` makes the result of each easy to collect. A `batch` command would duplicate the
shell. Per ADR 0017 (question 1), it is left out. Instead, `docs/book/src/scripting.md` (user guide) gives the
recipes:

- every project in a folder to G-code, with a summary of failures;
- one project at several stock thicknesses (`--set stock.thickness=…`);
- collect cycle times into a CSV (`plan --json | jq`);
- check every project before a release of the post-processors (`check-project`, `run`).

The recipes are written for POSIX shells and for PowerShell (Windows users).

## 6. Documentation

- `docs/book/src/scripting.md` (user guide), a new user document:
  - the envelope;
  - each command's keys;
  - path syntax;
  - `get` and `set`;
  - the schema and editor setup;
  - the recipes.
- It is linked from the README. `CLAUDE.md`'s command list gains `schema`, `get`, `set`,
  `check-project` and `--json`.

## 7. Dependencies

- **`schemars`** (MIT): derives the schema from the models, which is why it can't drift. It
  goes in `chipload-job`, the only crate whose types need it, behind a cargo feature that
  the CLI turns on. The GUI build stays without it.
- **A JSON Schema validator, dev-dependency only** (tests): `boon` (MIT OR Apache-2.0) is a
  candidate, small and without network features; it is checked when built. Nothing ships
  with it.
- `cargo deny` and `THIRD-PARTY-LICENSES.txt` are updated in the same commit.

## 8. Decisions

1. **Schemas for the project file, the tool library and the machine profiles.**
   **Recommended**: the three formats people generate or share. Alternative: the project
   file only.
2. **Generated with `schemars`**, not written by hand. **Recommended**: one source of
   truth, with descriptions from the doc comments. Alternative: a hand-written schema,
   which needs no dependency but drifts as the format changes.
3. **The schema states types and allowed values, not ranges and cross-field rules.**
   **Recommended**: those stay in validation and the problem list (one place). Alternative:
   ranges in the schema too, through schemars attributes, which means writing each rule
   twice.
4. **No `$schema` key in project files**; editors map `*.chipload` to the schema.
   **Recommended**: no format change. Alternative: an optional `$schema` field that saving
   keeps.
5. **One envelope with a format version (`chipload_json: 1`)** on every command, while
   today's `plan`, `inspect` and `check` keys keep their place. **Recommended**: scripts can
   rely on it and detect a change. Alternative: nest each command's output under `result`,
   which is tidier but breaks today's `plan --json` users.
6. **Generic `get` and `set` by path, plus `--set` for one run.** **Recommended**: every
   field becomes scriptable without a command per field, and future fields come free.
   Alternative: more dedicated `op …` commands.
7. **Paths count from 0, as in the file and in the problem paths.** **Recommended**: a
   problem's path can be passed straight to `set`. Alternative: from 1, as `--op` and the
   GUI number operations, which makes the paths differ from the file.
8. **No `batch` command; shell recipes in the docs instead.** **Recommended** per ADR 0017:
   the shell does it, in parallel too. Alternative: `chipload run a.chipload b.chipload …
   --out-dir DIR`. This revises the approved plan's "batch runs", so it is your call.
