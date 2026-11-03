# Spec: cutting data in the tools, slot and clearing

Status: draft (2026-11-03), for review. Replaces the values part of docs/specs/materials.md
(materials stay, as names).

## 1. Why

The user, 2026-11-03: "when creating the first pass in a pocket, there is a full diameter
load on the bit, then when we start on the rings, the load is only 0,4*d. It is the same
for profileing, there the load is often constant 1xd. I want to improve the 'material'
preset to reflect this. Right now it doe not feel coherent." And on where the numbers live:
"It feels a bit split to have some in materials and some in tools", agreed 2026-11-03: all
in the tools, a material only a name.

Today:
- **One set of numbers for different loads.** A material holds one feed and one step down
  per tool, and every operation uses them. A profile through solid stock is a full-width
  slot on every pass. A pocket is cleared inside out (pocket.md §3): its innermost rings
  cut full width, the rings after them only the stepover. A step down right for the rings
  overloads the first ring; one right for the first ring wastes the rings.
- **Two places for feeds.** A library tool has its own values (`Cutting`), used without a
  material, and a material has values per tool. They are the same kind of numbers.
- **No spindle speed with the feed.** The chip load is feed / (rpm × flutes); the feed is
  saved in the material, the speed stays on the tool. A feed proven at 18 000 rpm is
  another chip load at 12 000.

What matters for the load (general machining knowledge, e.g. any machining handbook on
radial engagement):
- The chip a tooth takes is about the feed per tooth while the cut is at least about 0.3 D
  wide (at 0.4 D it is 98 % of a slot's): the feed per tooth can stay the same for slot and
  rings. Below about 0.3 D the chip thins and the feed could go up (not in this spec, §10).
- The force on the bit and the chips to clear grow with the width times the depth of the
  cut. A slot (width D) at 2.5 mm deep takes about as much as a ring at 0.4 D and 6 mm deep.
  So what should differ between a slot and a ring is mostly the **depth**.

## 2. Cutting data: one table per tool

A library tool holds its **cutting data**: one row per material, and one row for **Any
material**, used when no material is chosen or the tool has no row for it. Each row has:

| Value | Unit | Used for |
|---|---|---|
| **Spindle speed** | rpm | the program's S for this tool (§6) |
| **Plunge feed** | mm/min | going straight down, drilling |
| **Slot feed** | mm/min | full-width cuts (§3) |
| **Slot step down** | mm | the depth of each full-width pass |
| **Clearing feed** | mm/min | cuts at a stepover (§3) |
| **Clearing step down** | mm | the depth of each clearing layer |
| **Stepover** | × D | the width of the clearing cut |

Every value but the speed is required in a row: a row is one tested way to cut that
material with that bit. The speed may be unknown (a row converted from a tool without one,
§8); such a row says so and leaves the tool's speed as it is. The *Any material* row is today's tool values (`Cutting`) with a spindle speed; a tool
may have no rows at all (as a tool without `cutting` today).

A **material** is a name and a uid in the library (`materials.json`), nothing else: what
you choose for a project or an operation. Its page lists the tools that have a row for it.

## 3. Which operations use which values

| Operation | Values |
|---|---|
| Profile | slot feed and step down (a finishing lap or skin keeps its own, profile.md) |
| Pocket, and its rest and floor passes | slot and clearing (§4), stepover |
| Follow path | slot feed and step down |
| Holes | plunge feed to drill; slot feed and step down to bore |
| V-carve | slot feed and step down (its step down when it steps, vcarve-step-down.md) |
| Surface | clearing feed and step down; its own wide stepover stays (surfacing.md §4) |

Every operation takes the plunge feed. The operation keeps its own copies of the values
(as today): a row only fills them in when a material is chosen, the tool changes, or an
operation is added (materials.md §2, which stays as it is apart from where the values come
from).

## 4. A pocket goes down in slot steps where it cuts full width

At each depth a pocket cuts its rings inside out. A ring with no ring inside it (a leaf of
the ring tree, pocket.md §3: the innermost ring, and each innermost ring where the pocket
splits into branches) cuts full width; every other ring cuts the stepover.

With a slot step down smaller than the clearing step down:
- The pocket's depths are the clearing layers: equal passes no deeper than the clearing
  step down, as today's `pass_depths`.
- At each layer, a **leaf ring** first goes down to the layer's depth in equal slot steps
  no deeper than the slot step down, from the layer above (or the top), at the slot feed;
  then the rings outward cut the whole layer at once, at the clearing feed.
- The entry (plunge, ramp or helix) goes down with the first slot step, as it goes down
  with a layer today.
- With the slot step down equal to or larger than the clearing one, every ring cuts as
  today: the same G-code.

Example, slot 2.5 mm, clearing 5 mm at 0.4 D, a pocket 10 mm deep: the innermost ring at
2.5, then 5 mm, the rings out to the wall at 5 mm; the innermost ring at 7.5, then 10 mm,
the rings at 10 mm. Two clearing layers instead of four.

A pocket cut in one spiral (pocket-spiral.md) or staying down between layers
(pocket-stay-down.md) does so as today after its leaf rings' slot steps. Rest passes and a
V-carve's floor pass are pockets made from their pocket (rest-passes.md), so the same holds
for them, with their own values.

## 5. What an operation stores

A pocket and a clearing pass (rest and floor) get two optional values beside today's:
- `slot_feed`, mm/min;
- `slot_step_down`, mm.

Today's `feed`, `step_down` and `stepover` are the clearing values. Missing slot values
mean the clearing ones: every project cuts as before, and it is an optional field with a
safe default, so no schema bump (CLAUDE.md, persisted formats). Saving writes them
explicitly.

The other operations keep their fields: a profile's `feed` and `step_down` are its slot
values, and so on (§3).

## 6. Spindle speed

The program's spindle speed belongs to the tool (one S per tool group), not to an
operation. So:
- Choosing a material **for the project** sets each project tool's spindle speed from its
  row for that material, with the operations' feeds.
- Choosing a material **for one operation** sets only that operation's feeds. When its row's
  speed differs from the project tool's, the operation's chip load is shown at the tool's
  speed, and its Material row says so: *Plywood's values are for 18 000 rpm; tool 1 runs at
  16 000*.
- *Save to …* saves the project tool's speed in the row with the operation's feeds.

## 7. Choosing and saving in the window and on the command line

**The tool's form** (library, *This computer > Tools*): a **Cutting data** section, one
folded row per material, *Any material* first; the summary reads `18 000 rpm · slot 1500 /
2.5 · clearing 1800 / 5 × 0.4`. Open, a row has the seven values. *Add a material…* adds a
row from the library's materials; a row can be removed.

**The material's form** (*This computer > Materials*): its name, *Make default*, and the
tools with a row for it, each a link to that tool's form. No values to edit there.

**The operation's form:** the Material list as today. Its feeds section shows what it cuts
with:
- a pocket: *First ring (full width)*: feed, step down; *Rings*: feed, step down, stepover;
- a profile, a path, a V-carve: *Feed*, *Step down*, with *Full-width cut* as the hover;
- the chip load beside each feed, at the tool's speed.

The status row stays: *From Plywood*, *Changed from Plywood* with **Save to Plywood** and
*Use Plywood's*. Saving writes the values the operation has into the tool's Plywood row:
a profile its slot values, a pocket both, a surface its clearing values; the row's other
values stay. A tool with no Plywood row gets one, the missing values from its *Any
material* row (or refused, naming them, when it has none).

**Command line:**
- `chipload tool cutting "6 mm 2-flute" --material Plywood --rpm 18000 --plunge-feed 500
  --slot-feed 1500 --slot-step-down 2.5 --feed 1800 --step-down 5 --stepover 0.4` sets a
  row (`--material any` for *Any material*); `--remove` removes it. `tool show` lists the
  rows; `--json` everywhere.
- `chipload material add/remove/list/show/use/save/default/export/import` as today, without
  values: `material set NAME --tool …` goes (`tool cutting` replaces it), and `material
  show` lists the tools with a row.
- `chipload set part.chipload operations.0.slot_step_down=2.5` as for any value.

## 8. The library files

- **`tools.json` version 4:** each tool's `cutting` becomes `cutting_data`, a list of rows
  `{ "material_uid": "3f9c…" | null, "rpm", "plunge_feed", "slot_feed", "slot_step_down",
  "feed", "step_down", "stepover" }`, null for *Any material*. At most one row per
  material.
- **`materials.json` version 3:** a material has `name` and `uid`; `default_material`
  stays. Its `tools` values are gone.
- **Converting a library** (one step, both files, decision 7): when `materials.json` is
  version 2, its values move into the tools as rows, and both files are written (each with
  its `.bak`, as `save_job` keeps one). The step is a pure function of the two old
  libraries (no I/O, no clock), tested on frozen samples of both. What it makes:
  - each material's values for a tool become that tool's row for the material: slot and
    clearing both take the old feed and step down (so nothing cuts differently), the speed
    the tool's own `rpm` (a tool without one: the row has no speed, §2);
  - a tool's own `cutting` becomes its *Any material* row the same way;
  - values for a tool uid not in the tool library are dropped and listed in the report, as
    removing a library tool drops them today.
- Loading `tools.json` version 3 alone (no old materials file) migrates as above for its
  own `cutting`.
- Exported tool and material files (`tool export`, `material export`) are the same formats
  and convert the same way on import.

Projects keep their format: their operations hold their own values, and their
`material` names a material by name and uid as before.

## 9. What doesn't change

- The G-code of every project and every golden: slot values start equal to the clearing
  ones, and a pocket only goes down in slot steps once they differ.
- The checker, the cycle-time estimate and the statistics work on the moves as before.
- The materials' place in the window (*This computer > Materials*, the operation's
  Material list, *Use for all operations*, *Make default*).

## 10. Not now

- Faster feeds at a light stepover (chip thinning below about 0.3 D, a finishing lap).
- The higher load in a ring's inside corners.
- A tool's flute length, to warn when a clearing step down is deeper than the flutes.
- Copies of the cutting data in a project's tools, so that a project carries its
  materials' values to another computer.

## 11. Decisions to review

1. **The cutting data lives in the tools**, one row per material plus *Any material*; a
   material is only a name. *Agreed 2026-11-03.*
2. **A row holds speed, plunge, slot (feed, step down) and clearing (feed, step down,
   stepover).** *Agreed 2026-11-03.*
3. **Which operations use which values** (§3): slot for profiles, paths, V-carves and
   bores; clearing for a surface; both for pockets and their passes. *Recommended.*
4. **A pocket's leaf rings go down in slot steps** to each clearing layer (§4); rest and
   floor passes alike. *Agreed 2026-11-03 for pockets; recommended for the passes.*
5. **Slot values on a pocket are optional fields, missing meaning the clearing values** (§5):
   no project schema bump, no G-code change. *Recommended.*
6. **The spindle speed** (§6): a project material sets the tools' speeds; an operation's
   own material only its feeds, with a note when the speeds differ. A converted row whose
   tool has no speed is kept and asks for one. *Recommended:* one S per tool, as the
   program has.
7. **The library is converted once, both files together** (§8), with backups and a report
   line. This is a step across two formats, where CLAUDE.md has one migration per format: it
   is still a pure function, tested on frozen samples. *Recommended:* the values have to
   move from one file to the other; doing it per file would lose them in between.
8. **The tool's form gets the table; the material's form only lists tools; `material set
   --tool` gives way to `tool cutting`** (§7). *Recommended:* one place to edit, as agreed.

## 12. Tests

- **Library conversion:** frozen `tools.json` v3 and `materials.json` v2 samples (in
  `tests/corpus/library/`) convert to the rows described in §8; values for an unknown tool
  are reported; the files and their backups are written once; a converted library cuts
  every corpus project to the same G-code.
- **Rows:** round trip of `tools.json` v4; one row per material; a row with a bad value is
  refused by name; *Any material* is used without a material or a row.
- **Pocket slot steps:** a square pocket 10 mm deep, slot 2.5, clearing 5: the innermost
  ring at -2.5, -5, -7.5, -10, the other rings at -5 and -10 only; feeds slot and clearing
  on the right moves; a pocket that splits in two branches steps both leaves; equal values
  give byte-identical G-code; the checker passes; rest and floor passes the same.
- **Spindle speed:** a project material sets the tools' speeds; an operation's own
  material with another speed shows the note.
- **Saving:** from a profile writes the slot values, from a pocket both; a missing row is
  made from *Any material* or refused with the missing values named.
- **CLI:** `tool cutting` set, show and remove, with `--json`; `material show` lists tools.
- **Window:** the tool's Cutting data rows, the pocket's two feed groups, the layout tests
  in every language.
- **Goldens:** unchanged.
