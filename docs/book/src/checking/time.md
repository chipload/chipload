# Cycle time and chip load

## Cycle time

After generating, the report gives the program's estimated time, split into cutting,
rapids and spindle start-up, with one line per operation. With several
[parts](../operations/parts.md), each part's time comes first, with its operations under it.
With several tools, it also gives a time per file and the number of tool changes. Hovering an operation in the
Browser shows its time, and so does the setup sheet.

- **Feeds as programmed:** the estimate uses the programmed feeds, and moves at the full
  feed from start to end.
- **Real time is longer:** a real machine slows down in corners and while accelerating.
  Take the estimate as a lower bound for jobs with many short moves.
- **Rapids** move at the machine's **max feed** (Machine). Without one, the
  estimate assumes 5 000 mm/min and says so: set the max feed for a better estimate.

`chipload plan part.chipload` prints the same, and `--json` gives it as numbers.

## Statistics of an operation

The last section of an operation's properties, **Statistics**, says what the operation
does, from the last run. Folded, it shows the operation's time. Open, it shows:

- **Time:** split into cutting and rapids.
- **Cutting:** the length of the feed moves that go sideways: lines, arcs, ramps and
  helices, measured along the move (a ramp counts its way down too).
- **Plunging:** the length of the feed moves straight down or up: plunges and drilling.
- **Rapids:** the length of the rapid moves (G0), up and down included.
- **Lifts:** how many times the tool goes up at rapid: between paths, and at each peck of
  a peck drill. Many lifts in an operation can mean time spent in the air.

The numbers are for all of the part's copies, and a rest pass or floor pass has its own.
After a change they are out of date: the section asks you to generate (F5), and says
*Generating…* while the live preview runs. An operation that is off says so.

The report ends its time lines with the whole job's lengths and lifts (*In all: …*), the
moves between operations included. `chipload plan` gives each operation's numbers on its
line, and `--json` has them for each operation and the job: `cutting_mm`, `plunge_mm`,
`rapid_mm`, `lifts`, and the seconds `cutting_s` and `rapid_s`.

## Chip load

For each operation, Chipload shows the **chip load**: how thick a chip each cutting edge
takes, in mm per tooth:

    chip load = feed / (spindle speed × flutes)

- **Too thin** and the tool rubs and heats up.
- **Too thick** and it may break.

The right value depends on the material, the tool and the machine, so Chipload shows the
number and doesn't judge it. As a rough guide:
- wood and plastics with 3–12 mm tools often take 0.05–0.25 mm per tooth;
- aluminium takes 0.02–0.08.

Check the tool maker's figures.

Setting it up:
- **Where:** in an operation's *Feeds*, on a *Chip load* row under the feed.
- **Flutes and spindle speed:** the tool needs both, on the tool (click it under *Tools* in
  the Browser) or in the tool library. Until then the row says which is missing.
- **Feed:** a drill's chip load uses its feed down; everything else uses the cutting feed.
