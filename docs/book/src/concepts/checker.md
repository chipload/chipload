# The checker

Every program goes through an independent checker before Chipload writes it, in the app and
on the command line. A program that fails isn't exported.

**What it is.** The checker is separate code that shares nothing with the code that makes
the toolpath. It reads the finished G-code line by line, as a controller would, and judges
it against the project's stock, safe Z and machine. Because it is independent, a bug in the
toolpath code is likely to be caught by the checker, and the other way round.

**What it refuses:**
- a rapid (G0) down into the stock, or sideways while the tool is below safe Z;
- a cut deeper than the stock bottom plus *Max depth below stock*;
- moves beyond the machine's travel, when the travel is set;
- an arc whose ends don't lie on its circle, or an arc that is a full circle in one move;
- a cutting move with no feed, a zero feed, or the spindle off;
- relative (G91) coordinates, and motion before G21 or G20 and G90;
- a program in other units than the machine's *G-code units*, or one that changes units
  part way (an inch program is read in inches and checked in mm);
- a tool change with the spindle running or below safe Z;
- any G or M code it doesn't know (see [Post-processors](../posts.md#your-own-post)).

**What it warns about:**
- cuts into the spoilboard, within the allowed depth;
- travel limits that weren't checked because the machine's travel isn't set.

**On any G-code:** `chipload check part.nc --job part.chipload` checks a file, also one
made by another program, against a project's stock and machine. With several tools, add
`--tool N` to check one file of a set.

**What it can't know:** your clamps and fixtures, the tool's real length, whether the stock
is really there, and whether the zero is set right. The [safety rules](../README.md#safety)
still apply.
