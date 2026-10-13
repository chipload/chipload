# Side and direction

## Side (profiles)

A profile cuts **outside** the line (the part is what's inside), **inside** it (a hole: the
waste is inside), or **on** the line (the tool's centre follows it, like a slot).

When you add a profile, Chipload picks *outside* for outlines and *inside* for holes. For a
selection with both, or with nothing to go by, it asks: the side is marked until you choose
one. It never guesses which side is waste.

## Direction

**Climb** or **conventional**, per operation:

- **Climb:** the cutter's edge enters the material at full chip thickness. This gives a
  better finish and less rubbing on rigid machines, and is the usual choice on CNC
  routers.
- **Conventional:** the edge enters at zero chip thickness. It is gentler on machines with
  backlash.

**What it means for the path.** With the spindle turning clockwise (M3, Job settings >
Machine > Spindle):
- climb goes **clockwise** round an outside profile and an island;
- climb goes **counter-clockwise** round an inside profile and a pocket's walls.

Conventional, or a counter-clockwise spindle, turns each way round. The arrows on the
canvas show the direction.
