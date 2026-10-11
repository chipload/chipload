# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

- **GUI pass:** make the GUI a little more attractive and intuitive (proposal approved
  2026-10-11, all eight points as recommended).
  - [x] The report under the canvas: result line, coloured warnings that select their
    operation, a Drawing section.
  - [x] Forms in sections with summaries; operations as cards.
  - [x] A toolbar row (Generate stands out) and a start screen.
  - [x] A canvas legend, an out-of-date notice, View > Theme, a calmer style.
  - [x] Remember the theme between sessions (settings schema 2, decided 2026-10-11).

## Done

- **Start points, lead-in/out, helical entry** (docs/specs/helix-and-leads.md), in 0.9.0.

## Later

- **Trochoidal vs adaptive:** decide whether trochoidal should get bigger loops that cover
  the pocket's width, or whether adaptive already does what's wanted.
- **Offset retry** (parked, docs/specs/offset-retry.md, draft): a pocket whose island
  offsets just touch at one exact stepover is refused; retry with a slightly smaller one.
