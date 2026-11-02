# Spec: one way to say where a shape comes from

Status: **draft 2026-11-02, for review.**

This is the user's review of 2026-11-02 ("where does the complexity lie? Any part that
feels like a hack?"), point 1, asked for after point 2 (the window holds the project
whole, done). The finding: where a shape comes from, and how an operation names "all of
this source but these", is written several different ways. Each new kind of source (texts,
then groups this week) had to be added to all of them, about 30 files for groups.

This is an internal change. Nothing is cut differently, the project file doesn't change,
and the command line and scripting paths stay as they are.

## 1. What is there now

**The origin of a shape** sits inside its geometry. `Geometry::Path` has five fields for it:
`dxf_handle`, `dxf_type`, `dxf`, `image` and `text`. Nothing in the type keeps more than one
of them from being set, or a handle from being set without a DXF. In practice they never
are: after loading, every one of the 276 paths in the test corpus has exactly one of these
four combinations:
- a DXF with its handle and entity type;
- a picture;
- a text;
- none of them (drawn).

The checks for this are spread through the code instead of being in the type.

**The selector** of an operation is seven optional fields: `layer`, `shapes`, `dxf`, `image`,
`text`, `group`, `exclude`. The type allows combinations that make no sense. Each is refused
by a separate check in `Job::validate`:
- two sources at once (`job-selector-one-source`);
- `exclude` without a source (`job-exclude-no-source`);
- a group together with shapes.

Three other places each repeat the "which source is it" match on the four fields:
- `select.rs` `matches`;
- `Selector::may_pick`;
- the summary in the window.

**The window's selection** has `Source(from)`, `Except(from, left_out)` and
`Group { id, shapes, left_out }`. These are three spellings of "the whole of this, but these
left out", and every handler of the selection has a branch for each.

**Placement** is different per kind:
- a DXF has a `Placement` that is applied when it is chained;
- a picture has `x, y, width, rotation`;
- a text has `x, y, height, rotation, anchor`;
- drawn shapes are changed in place;
- a group moves its members kind by kind.

These are really different settings: a text has a height, a picture a width. They stay as
they are (decision 6).

## 2. The change

### 2.1 Origin: one enum in the path

```rust
/// Where an imported, traced or lettered path comes from. None: drawn.
pub enum Origin {
    /// Read from a DXF or SVG: its id in `drawing.dxfs`, the entity's handle and type.
    Dxf { id: u64, handle: String, entity: String },
    /// Traced from the picture with this id.
    Image(u64),
    /// A letter of the text with this id.
    Text(u64),
}

Geometry::Path { closed, segments, origin: Option<Origin> }
```

`Shape::dxf()`, `image()`, `text()` and `dxf_handle()` stay as small accessors on top of
it, so most callers don't change. A new `Shape::source() -> Option<ShapeFrom>` gives core's
`ShapeFrom`, which the segments carry already.

**In the file**, the shape is written exactly as now: the flat keys `dxf_handle`,
`dxf_type`, `dxf`, `image` and `text`. Reading them maps to `Origin`. A combination that
isn't one of the four in §1 is refused with a typed error naming the shape: two sources, a
handle without a DXF, or a DXF without a handle.

Today such a file loads and cuts in a way nobody intended. Refusing it is the rule for
hostile input. No file Chipload has ever written has such a combination (decision 3).

### 2.2 Selector: a source with what's left out

```rust
pub enum Source { Dxf(u64), Image(u64), Text(u64), Group(u64) }

pub struct Selector {
    pub layer: Option<String>,
    pub shapes: Option<Vec<u64>>,
    /// All of a DXF, picture, text or group, but the shapes in `exclude`.
    pub source: Option<Source>,
    pub exclude: Vec<u64>,
}
```

**In the file**, a selector is written as now: `{"dxf": 2, "layer": "HOLES"}`,
`{"group": 1, "exclude": [6]}`. So `chipload get/set operations.0.select.dxf`, the JSON
schemas and every example in the guide keep working.

Reading refuses two sources at once, the same as `Job::validate` does now, but now at the
place where the selector is read. `exclude` without a source becomes impossible to
construct in Rust. In a file it is refused when read, as now.

What changes for each user of the selector:
- **`matches` in select.rs** and **`may_pick`** test one `source`, against the segment's or
  shape's `ShapeFrom`. A group is resolved to its shapes first, as now (`resolved`).
- **The validation** of which source exists, and of `exclude` naming that source's shapes,
  becomes one match on `Source` instead of four `if let`s and a 4-tuple.
- **`describe`** (the selector in words) and the window's summary each get one match.

### 2.3 The window: one "whole" selection

```rust
pub enum CanvasSelection {
    Nothing,
    Shapes(BTreeSet<u64>),
    Layer(String, Option<ShapeFrom>),
    /// All of a DXF, picture, text or group, but these shapes.
    Whole { of: Source, left_out: BTreeSet<u64> },
}
```

This replaces `Source`, `Except` and `Group`. A group's shapes are read from the drawing
when they're needed, not kept in the selection. Clicking, Ctrl+click, the box handles,
moving, the Browser and making an operation all work as now; the tests in
`app/tests/groups.rs`, `leave_out_shapes.rs` and `images.rs` must pass unchanged in what
they check.

`BoxOf` follows: `Source(ShapeFrom)` and `Group(u64)` become one `Whole(Source)`.

## 3. What doesn't change

- **The project file:** every corpus project is read and written back byte for byte the
  same (a new test). No schema bump.
- **G-code:** the golden files and setup sheets are unchanged.
- **The command line, MCP and JSON Schema:** unchanged. `chipload schema project` gives
  the same output (checked against `docs/schemas/`).
- **Messages:** unchanged, apart from the new refusal of a mixed-up shape origin (§2.1),
  in all six languages.

## 4. Order of work

Three commits, each with all tests passing:
1. `Selector` with `source` and `exclude` (§2.2), with its serde mapping and validation.
2. `Origin` (§2.1).
3. The window's `Whole` selection and `BoxOf` (§2.3).

Each one: the workspace tests, clippy, fmt, and the new byte-for-byte round trip of the
corpus.

## 5. Tests

- **Round trip:** every project in `tests/corpus/jobs/` (after its migrations), dumped,
  parsed and dumped again, gives the same text. Each current-version (v35) file gives back
  its own bytes.
- **Refused when read:**
  - a selector with two sources, or with `exclude` but no source;
  - a path with two origins, a handle without a DXF, or a DXF without a handle.
  Each gives a typed error with the path to the field.
- **Existing tests** for selecting, leaving shapes out, groups, pictures and texts pass
  unchanged.

## 6. Decisions

1. **Change the Rust types only; keep the file and the scripting paths as they are.**
   Recommended. The flat file form is readable and documented in the guide, and scripts use
   paths like `select.dxf`. The looseness is in the Rust types, not in the file. *Instead:*
   a nested `"source": {"dxf": 2}` with a schema bump. That breaks scripts and buys
   nothing.

   This corrects the review, which said a schema bump would be needed. Looking closer, it
   isn't.
2. **The origin goes inside `Geometry::Path`, not on `Shape`.** Recommended. Only paths
   come from a DXF, a picture or a text. A circle read from a DXF is a path today. *Instead:*
   on `Shape`, which would let a drawn rectangle claim to come from a DXF.
3. **Refuse a mixed-up origin when reading** (two sources, a handle without a DXF, or a DXF
   without a handle). Recommended: input is treated as hostile, and Chipload never writes
   such a file. *Instead:* take the first and ignore the rest, which silently changes what
   gets cut.
4. **A group is a `Source` in the selector and in the window, but not in core's
   `ShapeFrom`.** Recommended. Segments carry where they were drawn from, and a group isn't
   that; it's resolved to shapes before cutting, as now.
5. **`exclude` becomes a plain list (empty: nothing left out)** instead of an `Option` of a
   list. It is still written only when not empty, so the file doesn't change. Recommended.
   The one difference: a hand-written `"exclude": []` (no corpus file has one) is dropped
   the next time the file is saved. It means the same thing either way.
6. **Placement stays per kind.** Recommended. A text's height, a picture's width and a DXF's
   scale are different settings that users set. Joining them would be a file change for no
   gain. Moving a "whole" selection in the window goes through one function per kind, as
   now.
7. **Later, not now:**
   - the window's modes as one enum (review point 3);
   - splitting `models.rs`, the CLI's `main.rs` and `forms.rs` (point 4).
