# 0029: Cutting data in the tools, slot and clearing

Status: accepted (2026-11-03). Spec: docs/specs/cutting-data.md (agreed 2026-11-03,
decisions 1-8 as recommended).

## Context
Feeds lived in two places: a library tool's own values (`Cutting`, used without a
material) and a material's values per tool (`materials.json`). Both were one set of
numbers, used by every operation. But a bit cutting its full width (a profile, a pocket's
first ring) carries far more load than one cutting a stepover (a pocket's other rings), so
one step down was either too deep for the first or too shallow for the rest. The user:
"Right now it doe not feel coherent", and "It feels a bit split to have some in materials
and some in tools".

## Decision
- **The values live in the tools:** a library tool has `cutting_data`, one row per
  material uid and one for any material (`material_uid: null`). A material is a name and a
  uid. `tools.json` version 4, `materials.json` version 3.
- **A row has slot and clearing values:** speed (optional: the tool's own), plunge feed,
  slot feed and step down, clearing feed, step down and stepover. Each operation takes the
  ones it uses (`Operation::apply_cutting`).
- **A pocket goes down in slot steps where it cuts full width:** optional `slot_feed` and
  `slot_step_down` on pockets and clearing passes; missing, they are the feed and step down,
  so no project format bump and no G-code change.
- **The library is converted once, both files together** (`convert_library`): the old
  materials' values move into the tools (`move_material_values`, pure), then both files are
  written in their current versions with `.bak`. CLAUDE.md asks for one migration per format;
  here a value moves from one file to the other, so each file's own migration can't do it.
  Each file still has its own migration step (`tools_v3_to_v4`, `materials_v2_to_v3`), and
  the move between them is a pure function tested on frozen samples.

## Consequences
- One place to edit feeds: the tool's form (*Cutting data*), `chipload tool cutting`.
  `material set --tool` and `tool add --feed …` are gone.
- A project is unchanged: its operations hold their values, its materials are named by
  name and uid.
- The CLI and window convert the library before reading it; a library half converted (a
  failure between the two writes) converts again without duplicating rows.
