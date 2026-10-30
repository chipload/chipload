# Scripting Chipload

Everything Chipload does can be driven from scripts: the command line takes every action,
`--json` gives results a program can read, `get`/`set` change any value of a project, and
JSON Schemas describe the files. (Design: `docs/specs/scripting.md`.) An AI assistant can
drive the same commands through [MCP](assistants.md).

## JSON output

Add `--json` to any command, before or after it (`chipload --json plan part.chipload` or
`chipload plan part.chipload --json`). The command then prints exactly one JSON object on
stdout, on one line, and nothing else. This holds when it fails too.

Every object has these keys:

| Key | |
|---|---|
| `chipload_json` | `1`: the version of this format. It goes up if a key changes meaning or is removed. |
| `ok` | `true` when the command did what it was asked (exit code 0). |
| `notes` | Warnings and notes: the lines the command prints without `--json`. |
| `written` | Files written, absolute paths, in order. |
| `error` | Only when `ok` is false: `{"message": "…", "problems": [{"path", "short", "text"}, …]}`. |

`problems` are the project's problems, as the GUI shows them, each with the path of the
field it is about (`operations.0.rest`). Pass that path to `set` to fix the field.

The exit codes are the same with or without `--json`:
- **0:** done;
- **1:** failed, including a refused check or problems found;
- **2:** the command line itself is wrong (clap's message, not JSON).

### Each command's own keys

| Command | Keys |
|---|---|
| `plan` | `program`, `groups`, `pockets`, `material_left`, `ring_starts`, `tabs`, `estimate`, `chip_load` |
| `run` | `files` (`path`, `tool`, `operations`), `sheet` (path or null), `estimate` |
| `simulate` | `numbers` (`removed_cm3`, `deepest_z`, `below_bottom_mm`, `cell_mm`, …), `image` (path or null) |
| `inspect` | `units`, `issues`, `bbox`, `contours`, … (the import report) |
| `import` | `report` (as `inspect`) |
| `update` | `unchanged`, `changed`, `added`, `removed` (shape ids), `lost` (operation indexes) |
| `check` | `errors`, `warnings` (`line`, `message`) |
| `shape add`, `shape copy` | `ids` |
| `place` (no options) | `placement`, `size` |
| `get` | `value` |
| `set`, `check-project` | `problems` |
| `tool list`, `machine list` | `path`, `default`, `tools` / `machines` (as in the library file) |
| `tool use` | `id`, `added` |
| `tool import`, `machine import` | `added`, `same`, `differing`, `replaced` |
| `post list` | `path`, `posts` (`name`, `builtin`, `problem`) |
| `render` | `gcode` |
| `licenses` | `text` |

Operations and moves are indexed from 0 in JSON. Times are in seconds and lengths in mm.
Object keys come out sorted.

`schema` prints the schema itself, with or without `--json` (see below).

## Reading and changing values: `get` and `set`

A **path** is keys and array indexes joined by dots, as in the project file, counting from 0:
`stock.thickness`, `operations.1.end_depth`, `tools.0.rpm`. Note that `--op` on `op tool`
and `op rest` counts from 1, as Chipload numbers operations on screen.

The stock and the work zero belong to a setup (`setups.0.stock.thickness`,
`setups.0.work_zero`); `stock.…` and `machine.work_zero` still work and mean the active
setup's, so older scripts keep working.

```sh
chipload get part.chipload operations.1.end_depth          # 6.0
chipload set part.chipload operations.1.end_depth=6.5 operations.1.name=Corners
chipload set part.chipload 'operations.1.rest={"after_tool": 1}'
chipload set part.chipload operations.1.rest=null           # turn an optional setting off
chipload set part.chipload "operations.3=$(chipload get part.chipload operations.0)"   # with 3 operations: append a copy of the first
chipload check-project part.chipload                        # problems with their paths
```

How `set` works:
- **Values:** each value is JSON if it parses (`6.5`, `true`, `null`, `{"after_tool": 1}`),
  and text otherwise (`Corners`). For text that looks like JSON, quote it: `name='"6"'`.
- **Adding:** setting the index one past the end of a list appends to it. A missing optional
  field is added.
- **Checked before writing:** the whole project is loaded again before anything is
  written. A misspelt field, a wrong type, or a value the format refuses stops it, and the
  file stays as it was.
- **Problems don't stop it:** a project that loads but would be refused at generation (a
  rest tool smaller than the pocket's, say) is written, and its problems are listed, as
  the GUI keeps an edit with a red field.
- **Fixed fields:** `schema_version` and `written_by` can't be set.

For one run only, `run`, `plan`, `simulate` and `sheet` take `--set PATH=VALUE`, which you
can repeat. The project file isn't changed:

```sh
chipload run part.chipload --set stock.thickness=18 -o part-18mm.nc
```

## JSON Schemas

```sh
chipload schema project  > chipload-project.schema.json
chipload schema tools    > chipload-tools.schema.json
chipload schema machines > chipload-machines.schema.json
```

The same files are in this repository under `docs/schemas/`.

What they cover:
- **Current version only:** they describe the files at the current version. Older projects
  still load; Chipload upgrades them, and saving writes the current version.
- **What they check:** field names and types, required fields, and the allowed values of
  choices.
- **What they don't check:** ranges and rules between fields, such as "stepover between 0
  and 1" or "clearance less than the tool radius". Use `chipload check-project` for those.
- **Descriptions:** every field has one, taken from Chipload's source.

**VS Code** (completion and checking while editing a project by hand): in your settings,

```json
"files.associations": { "*.chipload": "json" },
"json.schemas": [
  { "fileMatch": ["*.chipload"], "url": "file:///path/to/chipload-project.schema.json" }
]
```

## Recipes

Chipload has no batch command: the shell does that better, in parallel too. These need
[`jq`](https://jqlang.org/) for the JSON.

**Every project in a folder to G-code, listing failures** (bash):

```sh
for p in *.chipload; do
  chipload run "$p" -o "${p%.chipload}.nc" --json |
    jq -r --arg p "$p" 'if .ok then empty else "\($p): \(.error.message)" end'
done
```

In parallel, four at a time:

```sh
ls *.chipload | xargs -P 4 -I{} sh -c 'chipload run "{}" -o "$(basename "{}" .chipload).nc" > /dev/null || echo "failed: {}"'
```

**One project at several stock thicknesses:**

```sh
for t in 12 15 18; do
  chipload run part.chipload --set stock.thickness=$t \
    --set operations.0.end_depth=$(echo "$t + 0.3" | bc) -o "part-${t}mm.nc"
done
```

**Cycle times into a CSV:**

```sh
echo "project,seconds" > times.csv
for p in *.chipload; do
  chipload plan "$p" --json | jq -r --arg p "$p" 'select(.ok) | "\($p),\(.estimate.total_s)"' >> times.csv
done
```

**Check every project** (for example after changing a post-processor):

```sh
for p in *.chipload; do chipload check-project "$p" --json | jq -r 'select(.ok | not) | .error.message'; done
```

**PowerShell** (Windows), every project to G-code:

```powershell
Get-ChildItem *.chipload | ForEach-Object {
  $r = chipload run $_.FullName -o ($_.BaseName + ".nc") --json | ConvertFrom-Json
  if (-not $r.ok) { "$($_.Name): $($r.error.message)" }
}
```

And in parallel (PowerShell 7):

```powershell
Get-ChildItem *.chipload | ForEach-Object -Parallel {
  chipload run $_.FullName -o ($_.BaseName + ".nc") --json | ConvertFrom-Json
} -ThrottleLimit 4 | Where-Object { -not $_.ok } | ForEach-Object { $_.error.message }
```
