# Profile

A profile cuts along a closed contour:
- **outside** it, to cut a part out;
- **inside** it, to cut a hole;
- **on** the line.

Select the contours, then **Profile** (ribbon, OPERATION). See [Side and direction](../concepts/side-direction.md)
and [Depths and oversize](../concepts/depths.md) for what every profile shares.

Open contours (centre lines, construction lines) in the selection are skipped with a note.
To cut along an open line, use [Follow path](path.md).

## The spiral ramp

A profile always goes down as **one forward spiral** along the contour: no plunge into
the part, so there is no plunge mark, and any endmill can cut it, centre-cutting or not.
In the operation's **Cut** settings, right after the step down, the **ramp angle** (3° by
default, at most 30°) says how steeply:

- each pass goes down at the ramp angle, then runs level for the rest of its lap, and never
  more than one step down per lap (on small contours the angle alone decides, so a pass
  may take more than one lap);
- at the end only the ramped stretch is cut level, so the floor is flat;
- the tool rises out along the cut path, never back and forth.

A steeper angle makes the ramp shorter: 4 mm at 3° takes 76 mm of contour, at 10° 23 mm.
In the file the angle is `ramp_angle`, in radians.

## Holding the part

**Holding** chooses how a part that is cut out stays in place: *none*, *tabs*, *onion
skin*, or *onion skin and tabs*. Only the chosen kind's settings show.

- **Tabs** keep the part attached at a few points until you cut it free.
- **An onion skin** holds it all round during the roughing; the last lap then cuts
  through the skin, and the part is free during that lap.
- **Both:** the skin holds it all round during the roughing, and the tabs keep it from
  moving while the last lap cuts through the skin.

## Tabs

Tabs are bridges of material left at the bottom of the cut, so a part cut out doesn't come
loose on the last pass.

| Setting | |
|---|---|
| **count** | tabs per contour, spread evenly along it |
| **width** | the bridge left on the part, along its edge (the flat top for sloped tabs) |
| **height** | the bridge's thickness, up from the bottom of the cut |
| **flank angle** | 90° for straight tabs (the tool goes straight up and down); less for sloped ones. A ramped profile gets 30° by default and follows the slope, which is gentler on the tool. |

- **Placement:** tabs slide off corners to straight stretches, by up to a quarter of their
  spacing.
- **Spacing:** they keep a tool diameter from each other and from the start of the contour.
- **Shown** highlighted on the canvas.

### Tabs placed by hand

You can put a contour's tabs where you want them instead. Select the profile after
generating: each tab has a small square on the canvas, hollow where Chipload spaced it and
filled where you placed it.

- **Drag** a tab along its contour; it generates again when you let go.
- **Right-click a tab** to remove it, or to space that contour's tabs evenly again.
- **Right-click the contour's toolpath** for *Add a tab here*.
- Under **Tabs** in Properties, *5 tabs placed by hand* counts them, and *Reset* spaces every
  contour's tabs evenly again. Each change is one undo step.

On the command line, list them and add or remove one at a point on (or near) the contour:

```
chipload op tabs part.chipload --op 2
ring 1  (146.551, 132.176)  spaced
…
chipload op tab part.chipload --op 2 --at 162,91            # add one
chipload op tab part.chipload --op 2 --at 146.5,132 --remove
chipload op tab part.chipload --op 2 --at 162,91 --even     # this contour spaced evenly again
chipload op tab part.chipload --op 2 --reset                # every contour
```

- The first tab you add or remove on a contour keeps its other tabs where Chipload had put
  them; from then on that contour has exactly the tabs you placed, and **count** is for the
  contours you haven't touched.
- A tab placed by hand stays where you put it, also on a corner (the report notes it).
- Generating stops, naming the tab, if two tabs come closer than their width plus two tool
  diameters, if a tab covers the contour's start (move the tab or the start), or if a tab
  is no longer on any contour of the operation (the drawing changed).
- The points move with the drawing when you [place](../drawings/placement.md) it or move,
  turn or scale the drawn shape, and every copy gets them. In the file they are
  `tabs.at`, points in drawing coordinates.

## Start point

Each contour's cut starts at a point Chipload picks. To move it, select the operation, then
**drag the start marker** along the contour on the canvas; right-click the marker to reset
it. Tabs Chipload spaces move with the start; tabs placed by hand stay. In the file,
`starts` holds the points.

## Finishing pass

**Finishing pass** with a *stock* value:
- the roughing laps leave that much on the wall;
- one light lap at the wall at full depth then takes it off.

The wall comes out clean: the tool isn't loaded on that lap, so it doesn't deflect, and
there are no step marks from the passes. The stock must be less than the tool radius.

## Onion skin

**Onion skin** with a *thickness*:
- the passes stop that far above the end depth;
- one last lap cuts through the skin.

Small parts stay held until the very end, on a vacuum table or instead of tabs. The skin is
at most one step down, and thinner than the tab height when there are tabs (*onion skin and
tabs* under Holding).

## Dogbones and T-bones

A round tool leaves a fillet of its radius in every inside corner, so a square tenon won't
go into the mortise. **Corners** reaches into them:
- **dogbones:** the tool goes into the corner along its bisector;
- **T-bones:** the tool goes in along the longer wall, so the notch is in the shorter wall,
  out of sight.

*up to* sets the sharpest corner angle that gets one (100° by default, so 90° corners
drawn slightly off count). A corner the tool can't reach at all (a notch narrower than the
tool) is left round, with a note.

A bone that also cuts into another contour that is cut (a thin wall between this cut and
another), or into another wall of its own (a short step next to the corner), is cut all the
same, with a warning that names the corner: check the part there, or choose another bone for
that corner. Lines that don't count:
- lines no enabled operation cuts (construction lines, a layer nothing cuts, a disabled
  operation's shapes);
- a line that meets what the operation cuts near the bone: on its own wall (a shared edge),
  crossing it, or, in a pocket, running into the pocket. It has been cut through there
  already.

### Corner by corner

Any corner can have its own choice, whatever **Corners** says: *round*, *dogbone*, *T-bone
along the longer wall* or *T-bone along the shorter wall*. It goes past *up to* too. So you
can turn one T-bone the other way, leave out the one corner where a bone doesn't fit, or
leave **Corners** on *round* and give bones only to the corners that need them.

With the operation selected, every corner on the tool's side has a small diamond on the
canvas: filled, with the tool's circle where its bone goes, when it gets one; hollow when it
stays round; amber when the last generation's tool couldn't reach into it. A ring round the
diamond marks a corner with its own choice. Click a diamond (or right-click it) for its
menu: *As the operation*, *Round*, *Dogbone*, *T-bone along the longer wall*, *T-bone along
the shorter wall*. Hovering an entry draws its bone; clicking it generates again. Each choice
is one undo step. Under **Corners** in Properties, *2 corners chosen one by one* counts them,
and *Reset* gives every corner the operation's setting again.

On the command line, list an operation's corners and what each gets, then choose:

```
chipload op corners part.chipload --op 2
(11.214, 301)  90.0°  dogbone
(511.214, 301)  90.0°  dogbone
…
chipload op corner part.chipload --op 2 --at 11.214,301 --bone tbone --along shorter
chipload op corner part.chipload --op 2 --at 511.214,301 --bone none
chipload op corner part.chipload --op 2 --at 511.214,301 --as-operation   # its choice removed
chipload op corner part.chipload --op 2 --reset                           # every choice removed
```

A choice is kept as the corner's point on the drawing, as start points are, and moves with
the DXF when you [place](../drawings/placement.md) it. Every copy gets it. If the drawing
changes so a chosen corner is gone, generating stops and says which choice to remove. The
setup sheet says how many corners were chosen one by one.

## In the file

`"type": "profile"` in the [project file](../reference/project-file.md#operation). Lengths are
in mm, feeds in mm/min, angles in radians.

| Field | Setting |
|---|---|
| `name` | Name |
| `enabled` | the tick on the operation's row in the Browser |
| `part` | the [part](parts.md) it belongs to |
| `select` | the geometry: `{"layer": "NAME"}` or `{"shapes": [ids]}` |
| `tool` | Tool: the project tool's id |
| `start_depth` | Start depth |
| `end_depth` | End depth |
| `side` | Side |
| `direction` | Direction |
| `clearance` | Oversize |
| `step_down` | Step down |
| `feed` | Feed |
| `plunge_feed` | Plunge feed |
| `ramp_angle` | Ramp angle, radians |
| `tabs` | Tabs: `count`, `width`, `height`, `flank_angle`, and `at` (tabs placed by hand, `[[x, y], …]`; left out when there are none), or null |
| `starts` | start points, `[[x, y], …]` in drawing coordinates |
| `finish` | Finishing pass: `{"stock": …}` or null |
| `skin` | Onion skin: `{"thickness": …}` or null |
| `corners` | Corners: `{"type": "dogbone"` or `"tbone", "max_angle": …}`, or null |
| `corner_choices` | corners with their own choice, each `{"at": [x, y], "bone": "none"` or `"dogbone"` or `"tbone", "along": "longer"` or `"shorter"` (T-bones only, else null)`}`; left out when there are none |
