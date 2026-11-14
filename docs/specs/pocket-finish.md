# Pockets: a finishing pass

Status: **approved 2026-11-14**, every decision as recommended (TODO *Next* item 9, from
the gap review of 2026-11-03; the user chose it for 0.37.0).

## 1. What it is for

A pocket today cuts its walls and floor with the same rings that clear it, at full load.
The tool deflects, so the walls come out a little undersize and carry a mark at every
depth pass, and the floor shows the rings' overlaps. For a fit (an inlay's pocket, a lid,
a recess for a hinge or a magnet) and for a clean floor, CAM programs rough with a little
material left, then take it off in light passes. Profiles have this since 0.14
(docs/specs/finish-and-skin.md); pockets get the same.

## 2. Format

An optional field on a pocket, left out when none (no schema bump; an older Chipload
refuses a pocket that has one):

```json
"finish": {"wall": 0.3, "floor": 0.2}
```

- `wall`: mm left on every wall by the roughing, outer walls and islands alike; 0 or
  more, less than the tool radius.
- `floor`: mm left above the end depth by the roughing; 0 or more, at most the step down,
  less than the cut (end depth minus start depth).
- At least one of the two is more than 0. A through pocket (`through`) takes no floor
  allowance, as there is no floor.

## 3. What is cut

For each separate area of the pocket (a *piece*), in this order:

1. **Roughing:** the pocket as today, with the walls `wall` mm farther in (a clearance
   smaller by `wall`) and the passes ending `floor` mm above the end depth. Slot feeds,
   ramps, helixes, spiral and rings are all as today.
2. **Floor** (when `floor` > 0): one level pass at the end depth over the roughed area,
   with the same rings and stepover as the roughing's last pass. The tool goes down inside
   the roughed area as the roughing's own entry does (it is only `floor` deep).
3. **Walls** (when `wall` > 0): one level lap of each wall ring, outer and islands, at
   the end depth, at the pocket's clearance as today's wall ring.
   - **Going down:** straight down beside the wall, in the roughed area, which is already
     cleared to the end depth (or to `floor` above it when there is no floor pass, then
     only that deep into material).
   - **Onto the wall:** with the pocket's lead when it has one: the quarter circle from
     the cleared side, which a pocket has room for, unlike a profile's slot. Otherwise
     straight across the `wall` mm.
   - Then the lap in the pocket's direction (climb or conventional), and off the wall the
     same way before the retract.
   - Ring starts dragged on the canvas (`starts`) apply to these laps.

Rest passes (rest-passes.md) still come after the pocket. They clear what the finished
pocket leaves, as before, since the finished walls are where today's walls are.

## 4. Corners

- **Dogbones and T-bones** (dogbones.md): the roughing cuts the bones at each depth pass as
  today, so the wall lap never plunges a whole bone at full depth. The wall lap follows the
  ring with its bones, now only a skim.
- **Round inside corners** are as today: the tool's radius.

## 5. Refused, and notes

- **Too narrow to rough:** where the finishing ring runs a tool radius or more from the
  roughing path, the roughing couldn't get there with the allowance. A neck between an
  island and a wall is between *D* and *D* + 2·`wall` wide, so the lap would cut it full
  width at full depth. The operation is refused, naming the place:
  `toolpath-pocket-finish-narrow`, "the pocket is too narrow near (x, y) to rough with
  0.3 mm left on the walls: lower the wall allowance, or use a smaller tool". This is the
  same check as the profile's finishing lap (finish-and-skin.md, §6 as built).
- **A rest pocket** (`rest`: only what a larger tool leaves) takes no finishing pass: it
  is already a light clean-up (`job-pocket-finish-rest`).
- A wall allowance at or above the tool radius, a floor allowance above the step down,
  floor with through, and both 0 are refused at their fields.

## 6. Command line and window

- **Command line:** `set PROJECT operations.N.finish={"wall":0.3,"floor":0.2}`, or `null`
  for none. Styles carry it like any other field.
- **Window:** a *Finishing pass* section in the pocket's properties: a tick, then *Wall
  allowance* and *Floor allowance*. Ticking it starts at wall 0.3 mm and floor 0.2 mm. The
  Statistics section counts the finishing laps with the rest.
- **Setup sheet:** the pocket's line says "finishing: walls 0.3 mm, floor 0.2 mm".
- **Guide:** the pocket page explains it next to rest passes.

## 7. Edge cases and tests

- Each allowance alone and both; through with a wall allowance only.
- A pocket with islands: every island's wall gets its lap; nothing gouges (the existing
  pocket proptest, with a finish: the path never goes past the finished wall, and the
  roughing never comes nearer than `wall` to it).
- Floor allowance equal to the step down, and the cut equal to one pass.
- Dogbones with a finish: the roughing's bones and the lap's.
- A lead and no lead; climb and conventional; dragged starts.
- Too narrow: refused with the place; just wide enough: cut.
- Rest passes after a finished pocket: the same moves as after an unfinished one.
- The checker passes every program (rapids, retracts).
- Goldens: one pocket with islands, dogbones and both allowances; existing goldens
  unchanged (no `finish`, no change).

## 8. Order of building

1. Format, validation, planning, the command line, golden.
2. Window, sheet, guide.

## 9. Decisions

1. **One operation, the same tool** (§2): no tool change. *Recommended.* A separate
   finishing tool can come later as a pass like rest passes, if wanted.
2. **Per piece: roughing, floor, walls** (§3): the wall lap last, so it also cleans the
   corner where wall meets floor. *Recommended.*
3. **The floor pass at the pocket's own stepover,** one level at the end depth. No
   setting of its own. *Recommended.*
4. **The wall lap at full depth in one level lap,** as a profile's finishing lap, with the
   pocket's lead when set. *Recommended.* The alternative, a lap per depth pass, gives the
   step marks back.
5. **Bones cut by the roughing, skimmed by the lap** (§4). *Recommended.*
6. **Refused when too narrow to rough, and on a rest pocket** (§5). *Recommended.*
7. **Defaults when ticked: wall 0.3 mm, floor 0.2 mm.** *Recommended.*
8. **No schema bump** (§2). *Recommended.*
