# Spec: a sign of a fixed size, and New sign… on the start page

Status: **approved 2026-11-22** (the user: "go ahead"; §6 as recommended; see *As
built*, §7). Two leftovers of the sign wizard (TODO, *Next* 8,
signs), done before 0.40.0 at the user's request.

Builds on sign-maker.md (the wizard, frames that follow their text) and text-on-arc.md §6
(round words).

## 1. A sign of a fixed size

**Today:** the sign is as large as its words make it. The letters' height is given, and the
frame, the borders and the board grow round them. A sign for a given place, such as a
300 × 100 mm door plate or a 200 mm round plaque, means trying heights until it fits.

**Wanted:** the sign's outer size given, the letters as large as fit.

- ***Size:* Fit the words / Fixed** in *New sign…* (decision 1). Fixed shows *Width* and
  *Height* in place of the letters' height. With the round frame (equal width and height)
  it shows one *Diameter*.
- **The fixed size is the frame's outer line,** the outline that is cut out. The board
  fitted round the sign adds its margin outside it, as today.
- **The letters' height** is the largest that keeps the words inside the innermost
  border's groove, with the margin, the gap and the borders taken off, as for a sign that
  fits its words. It is rounded down to 0.1 mm, and the wizard shows it as it changes
  ("Letters: 34.6 mm").
  - Straight words: their box, every line, within the space inside.
  - Round words (text-on-arc.md §6): each arc line within its span of the circle and the
    middle lines clear, as the round words' radius rule says, inside the circle.
- **How it's found:** the height is searched (halving) between 1 mm and the size itself,
  building the sign each time and comparing its frame with the size. A sign grows with
  its letters, so the search finds the largest that fits. It doesn't depend on a formula
  for each frame.
- **The frame keeps its size later** (decision 2): it records the size (sign-maker.md §4,
  "a frame drawn to a fixed size records its size instead and follows nothing"). Editing
  the words afterwards doesn't grow the frame. Words that no longer fit overlap the
  border, and the sign's text shows as it does.
- **Refused, with the reason:** a size whose space inside is gone (borders and margins
  larger than the size); words that don't fit at 1 mm; round words with a width and height
  that differ (as today: `pipeline-sign-round-oval`).

**On the command line:**

```
chipload sign add P "SMITH\n12" --size 300x100
chipload sign add P "THE OAK\nEST. 2026" --frame oval --round --size 200x200
```

`--size WxH` (lengths, units allowed) instead of `--height`; the two don't combine. The
report says the letters' height found.

## 2. New sign… on the start page

**Today:** the start page offers *New project*, *Import a DXF or SVG* and *Open*. A sign
starts with a new project, then *New sign…* from the WIZARDS group.

**Wanted:** ***New sign…*** on the start page, below *New project* (decision 3). It
starts a new project with the app's defaults (the same as `chipload new`: the user's
default machine and tools from the library when there are, the built-in ones otherwise),
skipping the New project dialog, and opens *New sign…* on it. The wizard's board, fitted
round the sign, gives the stock. Cancelling the wizard leaves the empty new project, as
*New project* would.

## 3. Tests

- **Fixed size:** a straight sign of 300 × 100 has its frame 300 × 100 and letters within
  0.1 mm of filling the space inside (one more 0.1 mm step doesn't fit); a round sign
  of 200; more lines give smaller letters; refusals (no space inside, round with unequal
  sides, words too long for 1 mm).
- **Later edits:** words changed after a fixed-size sign leave the frame's size as it is.
- **CLI:** `--size`, `--size` with `--round`, `--size` with `--height` refused.
- **Window:** the Size choice switching the fields; the letters' height shown; *New sign…*
  on the start page opening the wizard on a new project.
- **Golden:** none needed; the existing sign goldens stay unchanged.

## 4. Format

No change: the frame record already has a fixed size.

## 5. Order of work

One commit each: the fixed size (pipeline, command line, wizard, guide's signs.md, seven
languages), then the start page's button (window, guide's first page and signs.md, seven
languages).

## 6. Decisions (recommendations first)

1. **A Size choice in the wizard: Fit the words or Fixed,** with Fixed replacing the
   letters' height. *Alternative:* both fields at once, with the height as an upper limit,
   which is harder to read.
2. **The frame keeps its fixed size after the sign is made;** the letters don't fit
   themselves again when the words change. Refitting them would make a text's height
   depend on the frame, a second kind of link. *Alternative:* the letters refit on every
   change, which needs a new record in the format.
3. **New sign… on the start page skips the New project dialog** and takes the defaults,
   since the wizard sets the board. *Alternative:* the New project dialog first, then the
   wizard (one more step, and the board it sets is replaced by the sign's).

## 7. As built

- **Fixed size:** `SignRequest::size`, `SignMade::fitted`; `sign add --size WxH` (it
  prints the height found, `height` in `--json`). The search stops within 0.05 mm, then
  rounds down to 0.1 mm. A sign round shapes already drawn with a size gets a frame of
  that size round them, the shapes as they are. In *New sign…*: *Size: Fit the words /
  Fixed*, *Outer size* (W, H) or *Diameter* for round words, and *Letters* showing the
  height found by the preview.
