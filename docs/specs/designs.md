# Spec: designs, and a box maker first (drawing phase 4)

Status: **draft 2026-10-28, on hold** (the user, the same day: DXF export first; decisions
in §12 not yet reviewed).

The user (2026-10-28), on parameters: "this would be cooler if the entire drawing could be
parametrized to create a cabinet of a certain size … So that box can be created just giving
width height length and thickness of material. The box should have nice corner joints of
course." And: "Could it be extensible so new designs can be created/added as well?"
Chosen as phase 4, before parameters (docs/specs/parameters.md, drafted, comes next and can
drive a design's values).

A box is the most common thing a maker cuts from sheet. Drawing its six panels with finger
joints by hand is slow and easy to get wrong, and every change of size or material means
drawing it again. A **design** draws a thing from a few numbers and draws it again when a
number changes. The box is the first design; the framework is built so that more can be
added (a drawer, a tray, a shelf, a sign blank) without changing the file format, the
command line or the window.

Clean room (CLAUDE.md): the joint layout below is worked out from the geometry of a box
here, not taken from any box generator's code or output.

## 1. What a user can do

- **Make a design** (a box) from its values: size, material thickness, a few joint
  settings. Chipload draws its pieces flat on the stock, ready to cut, and adds the
  operation that cuts them out, with dogbones in the joints' inside corners so the fingers
  close on a router.
- **Change any value** (a bigger box, thicker plywood, a looser fit) and the pieces are
  drawn again; the operation follows them.
- **Move** the whole design on the stock, and **explode** it into drawn shapes to change it
  by hand (as a DXF or a text).
- **See which designs there are** and what values each takes (`chipload design kinds`,
  Draw > Design).

## 2. Designs: how they are built (decision 1)

A design is a small piece of Chipload that **declares its values** and **draws its pieces**
from them:

- **Values:** each has a name, a kind (a length in mm, a whole number, a choice among
  words, yes/no), a default (which may depend on the project: "the stock's thickness"),
  limits, and a text in the six languages. The command line's options, the window's form,
  `--json`, the JSON schema and, later, parameters all work from these declarations alone.
- **Drawing:** values in, closed outlines out, in the design's own coordinates, with a
  name per piece (*bottom*, *front*, …) and the operations it suggests. Geometry only, in
  `chipload-draw` (a `designs` module, one file per design, on core's lines and arcs); no
  project model.
- **Checked:** every design checks what it drew before it is kept (for the box, §5).
- **Its own version:** a design records the version of its drawing rules with each use.
  When a later Chipload draws a design differently (a fix), projects made before keep
  their pieces as they are until a value is changed, and the window says that drawing it
  again will change it. A project never cuts differently without a word.
- **Adding a design** is one module with its declarations, drawing and tests, its texts,
  and a guide page; nothing else changes.
- **Designs written by users** (without building Chipload) would need a small sandboxed
  script language: deterministic, no files, no network. That is a separate spec and
  decision, later (§11).

## 3. A design in the project file

`drawing.designs` is a new list, empty by default, beside `texts`. Each entry has:

| field | meaning |
|---|---|
| `id` | its id, which its shapes and selectors name (`"design": id`) |
| `kind` | which design: `"box"` |
| `version` | the version of the design's drawing rules it was drawn with |
| `name` | its name in the Browser and messages (default `box 1`, …) |
| `values` | its values by name, as the design declares them: `{"width": 300, "top": "open", …}`; all written, defaults included |
| `x`, `y` | where its laid-out pieces start on the stock: their lower-left corner, mm |
| `layer` | the layer its shapes go on (default: its name) |

- **Its shapes** are `path` shapes on its layer naming it (`"design": id`), one closed
  outline per piece, named after the piece. They keep their ids when the design is drawn
  again (matched by piece name), so operations and their chosen starts, corners and tabs
  follow.
- **Selector:** `"design": id` picks the whole design, as `"text": id` picks a text.
- **Format version:** an optional list with a safe default, as `texts` was: no schema bump.
  Older Chiploads refuse a project that has a design ("unknown field"). A new design kind
  needs no format change; a Chipload that doesn't know a kind keeps its pieces as stored
  and refuses only to draw it again.

## 4. The box: values and panels

| value | meaning |
|---|---|
| `width`, `depth`, `height` | along X, Y and Z when it stands assembled, mm |
| `measure` | `outside` (default): the sizes are the box's outside; `inside`: the space inside |
| `thickness` | the material's thickness, mm (default: the stock's) |
| `finger` | the finger width aimed at, mm (default 3 × thickness) |
| `fit` | how much wider a slot is than its finger, mm in all (default 0.2) |
| `top` | `closed` (six panels) or `open` (five, no top) |
| `gap` | the space between laid-out panels, mm (default: twice the project's largest tool diameter, at least 10) |

With outside measures W × D × H and thickness t:

| panel | size (before joints) | joins |
|---|---|---|
| bottom, top | W × D | all four edges |
| front, back | W × H | bottom and top along W, left and right along H |
| left, right | D × H | bottom and top along D, front and back along H |

With `measure: inside`, W, D and H are the inside plus 2t. An open box has no top; the
upper edges of the four sides are then straight.

## 5. The box's joints (decision 4)

Every edge where two panels meet is a finger joint along its whole length L (the outside
length of that edge):

- **Fingers:** L is split into an odd number n ≥ 3 of equal pieces, n the odd number
  nearest L / `finger`. Odd, so both ends of an edge are alike. A piece is s = L / n long.
- **Which panel has the finger:** along an edge, the pieces alternate between its two
  panels: one has the even pieces (0, 2, …, the ends), the other the odd ones. A piece one
  panel has is a finger t deep on it, and the same piece is a slot t deep in the other.
- **Who has the ends** (so that each corner of the box belongs to exactly one panel):
  - bottom and top have the ends of all their edges;
  - front and back have the ends of their edges with left and right;
  - left and right have no ends.
- **Corner squares:** at a panel's corner, the t × t square is kept only when both its
  edges keep their end piece. With the rule above, each of the eight corner cubes of a
  closed box is in exactly one panel (bottom or top); on an open box, the four top corners
  are in the front and back.
- **Fit:** each slot is `fit` wider than its finger, `fit / 2` on each side, by narrowing
  the slot panel's fingers next to it. Depths stay exactly t, so the faces come out flush.
- **Too small:** a piece shorter than t + `fit` is refused ("the box is too small for
  fingers this wide": a smaller `finger`, or a bigger box).
- **Checked:** before the panels are kept, the two edges of every joint are tested to fit
  together (each finger of one in a slot of the other, with the fit), and every corner cube
  to belong to exactly one panel.

## 6. Corners a round tool can't cut (decision 7)

A router bit leaves the inside corners of every slot round, and a square finger won't go
in. The box's operation (§8) uses the profile's **dogbones** (docs/specs/dogbones.md):
every inside corner gets a relief of the tool's radius, whatever tool cuts it. The panel
outlines themselves stay exactly square, so the same box can go to a laser, or to another
tool, unchanged. *T-bone* corners (the relief along one wall only, hidden in the joint) are
the other choice on the operation, as for any profile.

## 7. Layout on the stock (decision 8)

The pieces lie flat, outside face down, in one row from (`x`, `y`) with `gap` between them
(for the box: bottom, front, back, left, right, top). A row that runs past the stock's
width wraps to a second row above. The stock-fit note says when the design doesn't fit.
Moving the design (dragging it, or `design set --at`) moves all its pieces; pieces aren't
placed one by one in this phase (nesting is out of scope, ADR 0017).

## 8. The operation it brings (decision 9)

Making a design also adds, in the active part, the operations the design suggests. For the
box: a **profile** of it (`"design": id`), outside, through the stock (the stock's
thickness, with the usual overcut), with dogbones, holding tabs on each panel, the
project's first tool. It is an ordinary operation from then on: change, move or delete it
like any other. `--no-operation` (and an option in the window) makes the design alone.

## 9. Command line

```
chipload design kinds                                   # the designs and their values
chipload design add P box --value width=300 --value depth=200 --value height=150 \
                    [--value thickness=18 …] [--at 10,10] [--name drawer] [--no-operation]
chipload design set P 1 --value width=320              # drawn again
chipload design list P
chipload design remove P 1                             # its shapes too; operations flagged
chipload design explode P 1                            # its pieces become drawn shapes
```

Values take units as every length option does (`--value width=12in`). `--json` gives the
kinds with their declarations, so scripts and the MCP server see them too.

## 10. The window

- **Draw > Design…** (and a ribbon button in DRAW) opens a window: the designs to choose
  from, then a form made from the chosen design's values, with a preview of the pieces
  where they will go, and *Create*.
- **The Browser** lists designs under *Drawing*, each with its pieces. Selecting one shows
  its values in Properties, made from the same declarations, where they can be changed
  (the pieces are drawn again as you type, as a text's letters are).
- **Right-click:** explode, remove.

## 11. Later, not now

- **More designs:** a drawer (a box with a false front), a tray, dividers, a shelf with
  dados, a sign blank with a rounded border.
- **Box options:** lids (sliding, lift-off), dividers, other joints (dovetails, rabbets,
  dowel holes), kerf for lasers.
- **Designs written by users:** a sandboxed script language, its own spec.
- **Parameters** (docs/specs/parameters.md), once built, can drive a design's values:
  `thickness = 18` sets the box and the depths together.

## 12. Decisions

1. **Designs as a family,** the box first: each declares its values and draws its pieces;
   the file format, command line and window are generic. *Recommended: yes*: the next
   design costs one module.
2. **A design is a generator object** (`drawing.designs`), drawn again when a value
   changes, as a text is; explode it to edit by hand. *Recommended: yes*: a box stays a box
   you can resize, which is the point.
3. **A design keeps the version of its drawing rules,** and a project is not drawn again
   under newer rules until a value changes (and then with a note). *Recommended: yes*: the
   same project gives the same G-code after an update of Chipload.
4. **The box's joint layout of §5:** odd finger counts; bottom and top own the box's
   corners, front and back the vertical edges' ends. *Recommended: yes*: every corner
   belongs to exactly one panel, and the rule is simple to check.
5. **Outside measures by default,** `inside` as an option; **fingers aimed at 3 ×
   thickness,** rounded to an odd count per edge. *Recommended: yes.*
6. **Fit 0.2 mm in all by default,** in the slots, depths exactly the thickness.
   *Recommended: yes*: snug in plywood; it is one number to change.
7. **Dogbones from the operation, not drawn into the panels.** *Recommended: yes*: the
   relief always matches the tool that cuts it, and the drawing stays true for a laser.
8. **Pieces laid out in a row with a gap,** moved as one design. *Recommended: yes.*
9. **Making a design adds its cutting operations** (for the box: profile, through,
   dogbones, tabs). *Recommended: yes*: ready to cut; ordinary operations afterwards.
10. **User-written designs later,** in their own spec (a sandboxed script language).
    *Recommended: yes*: built-in designs first, to learn what a design needs.
11. **Scope:** designs for common maker objects are within the drawing scope of ADR 0017 as
    amended (a maker would otherwise leave for another program to draw one). An ADR notes
    it. *Recommended: yes.*
