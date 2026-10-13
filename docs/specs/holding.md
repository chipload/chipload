# Spec: holding the part (tabs and onion skin as one choice)

Status: approved 2026-10-13 (the user: "Yes").

## 1. Why

User question (2026-10-13): "Also tabs and onion skins is similar, we would not want both?"

Both keep a cut-out part in place. In Chipload the onion skin isn't broken off by hand:
the roughing stops a thin skin above the end depth, and a last lap, after every ring of the
operation, cuts through it (docs/specs/finish-and-skin.md). So:

- **Onion skin alone:** the part is held all round during the roughing (small parts, a
  vacuum table, less vibration), and it is free once the last lap has gone round.
- **Tabs alone:** the part stays held at a few points until it is cut free by hand.
- **Both:** the skin holds it all round during the roughing, and the tabs keep it from
  moving while the last lap cuts through the skin. The skin must already be thinner than
  the tabs.

So both together is a real choice, but a rare one. Today tabs are a section of their own
and the skin sits with the finishing pass, so it isn't clear that they do the same job.

## 2. The form

- **A Holding section** after Cut takes the place of the Tabs section. Its first row is a
  choice:
  - *None*;
  - *Tabs*;
  - *Onion skin*;
  - *Onion skin and tabs*.
- **Only the chosen kind's rows show:** tabs (count, width, height, flank angle) and the
  skin's thickness.
- **Hover text** says what each does, including "the last lap cuts through the skin: add
  tabs if the parts must not move then".
- **The Finish and skin section** becomes **Finishing pass** (the finishing lap alone).
- **New values,** as today:
  - tabs 4 × 8 mm, 3 mm high, with the flank by the entry;
  - a skin of the step down or 0.3 mm, whichever is less.
- **Switching** from one kind to another keeps what stays chosen. Going from *Tabs* to
  *Onion skin and tabs* keeps the tabs as they were.

## 3. Decisions

1. **One choice of four,** with both still allowed. *Recommended,* because both is the
   only way to hold small parts through the skin lap.
2. **No change to the file format** (`tabs` and `skin` stay as they are), and no change
   to the G-code.

## 4. Tests

- Each of the four choices draws and sets `tabs` and `skin` as named; switching keeps the
  kind that stays.
- A problem on `tabs…` or `skin…` marks the Holding section.
