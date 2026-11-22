# Spec: laser and plasma

Status: **draft 2026-11-22**, for review. The user: "I'd like laser and plasma, basically it
is profile cutting and possibly marking." Settles TODO item 7 (CLAUDE.md promises G-code
for "CNC routers, mills, plasma, laser"; the posts are spindle-only). Builds on
tool-per-operation.md, tool-library-and-machines.md, follow-path.md, fill-pattern.md and
inch-units.md.

## 1. What is missing today

Every tool Chipload knows is a spinning cutter, and every program assumes one: M3/M4 with
an rpm, Z plunges and ramps, a material-removal simulation of a round tool. A laser or a
plasma torch cuts with a beam or an arc. It has no rpm and doesn't ramp. It must be switched
**off for every travel** and **on at every cut**, and what it removes is a line as wide as
its **kerf**. A laser is set by its power, a torch by its heights and pierce delay.

What users want from both, as the user put it:
- **Cutting profiles:** parts and holes cut out of sheet, the kerf on the scrap side, holes
  before the outline they sit in.
- **Marking:** lines (text, part numbers, bend and drill marks) burnt or scribed, not
  through. With a laser, also areas filled with lines.

## 2. What it does

- **Two new tool kinds**, `laser` and `plasma`, in the tool library and in projects like any
  tool. A tool holds the process settings for one material and thickness, as a
  manufacturer's cut chart does: "Plasma 45 A, 3 mm steel", "Laser 40 W, 3 mm plywood".
  - **Both:** `kerf` (mm, the width of the cut; it takes the place of a diameter) and the
    feed.
  - **Laser:** `power` (% of the machine's maximum) and `passes` (how many times each line is
    cut, for thick stock). The laser stays at one height (§4).
  - **Plasma:** `pierce_height`, `pierce_delay` (s), `cut_height` (mm above the sheet). A
    marking tool has no pierce delay.
- **Operations with a laser or plasma tool:**
  - **Profile:** outside, inside or on the line, offset by half the kerf, cut through in one
    pass (a laser: `passes` times at the same height). It has a **lead-in** and a
    **lead-out** on the scrap side (§5). No ramp, no tabs, no finishing lap, no onion skin.
  - **Follow path:** marking along lines, at the tool's settings. Its depth is ignored.
  - **Pattern (laser only):** an area filled with lines at a spacing, for marking. Its depth
    is ignored.
  - **The others** (pocket, V-carve, chamfer, flute, prism, moulding, holes, surface,
    relief) refuse a laser or plasma tool with a reason ("a laser cuts lines: use Profile,
    Follow path or Pattern").
- **Cut order:** inside before outside, as profiles already order cuts. A hole is cut before
  the outline round it, so the part doesn't drop or shift first.
- **In the window:** the tool form shows the fields of the kind chosen. The operation list
  offers only the operations the tool can do. The simulation shows a beam's cut as a groove
  of the kerf's width through the sheet. Marking shows as thin lines.
- **Machine profiles:** a laser machine is a profile with a laser post (§6), the same as any
  machine today. A router with a laser module uses two machine profiles, or one project per
  head. A program never mixes a spindle and a beam (§3).

## 3. Where "laser" lives (decision 1)

**In the tool.** A tool's kind says whether it spins, burns or arcs. The machine profile
names a post that supports that kind (`kinds = ["spindle"]`, `["laser"]`, `["plasma"]` in
the post's config). A project whose tools are of a kind its post doesn't support is refused
with a reason. With tools of several kinds (a router bit and a laser module), each kind goes
in its own file (`tool_change = files`). A program never switches between a spindle and a
beam: on GRBL that takes a setting change (`$32`), and the program can't make it safely.

*Alternative:* a `kind` on the machine (router, laser, plasma), every tool of that kind.
That is simpler to explain, but a router with a laser module would need two projects.

## 4. Switching the beam (decision 2)

- **Laser:** the beam is **off (M5) before every travel** and **on at the start of every
  cut** (M4 S<power>, GRBL's dynamic power in laser mode, which scales power with speed in
  corners). This holds even where laser mode would turn it off on G0 by itself: a machine
  that isn't in laser mode, or that has the wrong setting, must not burn a line across the
  sheet. No Z words: the focus is set by hand, and the program says so in its header.
  `S` is the power as a share of the post's `laser_max` (GRBL's `$30`, default 1000).
  *Alternative:* a focus Z for machines with a motorised Z. That could come later as an
  optional tool field.
- **Plasma:** each cut is: rapid to the pierce point at safe height, down to
  `pierce_height`, torch on (M3), wait `pierce_delay` (G4), down to `cut_height`, cut,
  torch off (M5), up to safe height. Torch height control (THC) during the cut is the
  controller's job: Chipload writes the cut height, and the THC takes over. Touching off on
  the sheet (a probe or a floating head) is written by the post's template when the post
  asks for it (`touch_off = true` in its config, G38.2 to the sheet before each pierce).
  *Alternative:* no Z at all, for tables with a manual torch height. That could come as a
  post setting.

## 5. Lead-in and lead-out (decision 3)

A pierce leaves a crater bigger than the kerf, and a cut that starts and ends at the same
spot leaves a nick. Every laser or plasma profile therefore starts on the scrap side and
runs into the line:
- **Lead-in:** an **arc** of a length set on the operation (plasma 4 mm, laser 1 mm by
  default), tangent to the cut at its start, the pierce on the scrap side: outside for an
  outer profile, inside for a hole. A hole too small for the lead-in gets a shorter one, and
  a straight lead from its centre when even that doesn't fit, with a note.
- **Lead-out:** an **overcut** past the start (0.5 mm default) to close the cut, then the
  beam off. No lead-out arc.
- **Where it starts:** the profile's start point as today (start.rs). The default is the
  middle of the longest straight side, away from corners, which burn on a plasma.

*Alternative:* a straight lead-in (perpendicular to the cut). It is simpler, but it leaves a
bigger mark on the part with plasma.

## 6. Posts (decision 4)

Written from the controllers' own public documentation, as all posts are:
- **GRBL laser** (`grbl-laser`): GRBL 1.1 in laser mode (`$32=1`), M4 dynamic power, S out
  of `laser_max`. For grblHAL, FluidNC and diode laser modules on GRBL routers.
- **Plasma** (`plasma`): plain M3/M5 with Z heights and G4 pierce delays, which Mach3/Mach4
  with a THC and most hobby plasma controllers read. `touch_off` off by default.
- **LinuxCNC plasma (QtPlasmaC)** only if its manual makes the torch commands and its
  material table clear. Otherwise it waits, as UCCNC does.
- **User posts** get the new template values: `kind`, `power`, `laser_max`,
  `pierce_height`, `pierce_delay`, `cut_height`, and each cut's start and end (`s.cuts`, beam
  on and off) next to the moves.

## 7. The checker (independent, decision 5)

`chipload-check` learns the kind from the job (`--job`), as it learns the limits now, and
checks the program, not the generator:
- **Beam or torch off during every G0.** M5 must come before any rapid after M3/M4. This is
  the one rule that matters most.
- **Laser:** no Z moves, and S never above `laser_max`.
- **Plasma:** Z never at or below 0 (the sheet) while moving sideways; a G4 after every M3.
- **Both:** feeds above zero and inside the machine's travel, as today. "Retract before XY
  travel" and "no rapid through material" are replaced by the beam-off rule. A beam has no
  stock to plunge into.

## 8. Format and model (decision 6)

- **Schema 44:** `ToolType` gains `laser` and `plasma`. A tool gains optional `kerf`,
  `power`, `passes`, `pierce_height`, `pierce_delay` and `cut_height`. A profile gains
  optional `lead_in` and `overcut`, used only with a beam. The migration from 43 changes
  nothing (no old file has a beam tool). The bump is there so an older Chipload refuses a
  laser project as "made by a newer version" rather than with an unknown-value error.
- **Tool library and machine profiles:** the same new tool fields. A post config gains
  optional `kinds` (default `["spindle"]`), `laser_max` and `touch_off`, with no bump of
  the post format.
- **Validation:** a beam tool needs its kind's fields. Kerf, power, passes and heights must
  be positive; power is 100 % at most; `cut_height` must not be above `pierce_height`.

## 9. Left out for now

- **Bridges (micro-joints),** small uncut gaps that hold parts in the sheet: a later step,
  like tabs.
- **Raster engraving of pictures** (dot by dot, grey levels): a different kind of program,
  and large. Marking with lines and Pattern fills comes first.
- **Laser power ramping in corners** beyond GRBL's M4, **kerf tables per hole size**, and
  slower feeds for small holes (plasma "hole cutting"): later, when a user asks.
- **Common-line cutting** and nesting tuned for beams: nesting works as today, with the
  kerf as the gap.

## 10. Decisions (recommendations first)

1. **Laser and plasma are tool kinds** (§3). *Alternative:* a kind on the machine.
2. **Laser: M5 before every travel and no Z; plasma: pierce height, delay and cut height,
   with THC left to the controller** (§4). *Alternatives:* a focus Z; no torch Z.
3. **An arc lead-in and an overcut** on every beam profile (§5). *Alternative:* a straight
   lead-in.
4. **Two built-in posts: GRBL laser and generic plasma.** QtPlasmaC only if its manual is
   clear (§6).
5. **The checker's beam-off rule** replaces the retract rules for beams (§7).
6. **Schema 44** for the new tool kinds (§8). *Alternative:* no bump, since an older
   version refuses the unknown kind anyway, though with a less clear message.
7. **Marking:** Follow path for lines with both, and Pattern fills with a laser only.
   *Alternative:* lines only at first.

## 11. Tests

- **Toolpath:** a laser profile is offset by half the kerf on the right side. The lead-in
  arc lies on the scrap side and is tangent to the cut; the overcut runs past the start.
  A small hole gets the short or straight lead-in. Holes are cut before their outline.
  With `passes = 3` each line is cut 3 times. Proptest: a lead-in never crosses the part.
- **Checker:** a G0 with the beam on is refused. So are a laser Z move, S above the
  maximum, and a plasma M3 without G4. Every generated beam program passes.
- **Refusals:** a pocket or V-carve with a laser tool. A laser tool on a spindle post. A
  laser and a router bit in one `program` file.
- **Goldens:** a laser plate (outline, holes, marked text) and a plasma bracket (outline,
  holes, overcut), both mm and inch.
- **CLI:** `tool add "Laser 40W ply" --type laser --kerf 0.15 --power 80 --passes 2`,
  `tool add … --type plasma --kerf 1.5 --pierce-height 3.8 --pierce-delay 0.5
  --cut-height 1.5`, `op add … profile` with a beam tool, `op set --lead-in 4`.

## 12. Order of work

1. Model and schema 44, tool kinds and fields, validation, CLI `tool add/set`, refusals for
   the other operations.
2. Beam profiles (kerf offset, lead-in, overcut, passes), follow path and pattern with a
   beam, cut order. The toolpath plan's cuts with beam on and off.
3. Posts `grbl-laser` and `plasma`, template values, the checker's beam rules, goldens.
4. The window: tool form fields, the operation list per tool kind, simulation and canvas
   drawing, the guide's new page *Laser and plasma*.
