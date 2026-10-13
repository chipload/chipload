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
- **Rapids** move at the machine's **max feed** (Job settings > Machine). Without one, the
  estimate assumes 5 000 mm/min and says so: set the max feed for a better estimate.

`chipload plan part.chipload` prints the same, and `--json` gives it as numbers.

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
- **Flutes and spindle speed:** the tool needs both, on the tool (click it in the Browser) or
  in the tool library.
- **Feed:** a drill's chip load uses its feed down; everything else uses the cutting feed.
