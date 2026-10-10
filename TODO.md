# TODO

What's planned, in order. Each item gets a spec in `docs/specs/` that is reviewed before
it's built (see `CLAUDE.md`).

## In progress

- **Start points, lead-in/out, helical entry** (docs/specs/helix-and-leads.md, approved
  2026-10-10). In this order:
  - [x] A. Start points: drag where each contour's cut starts.
  - [x] B. Lead-in/out: go down in the waste and arc onto the path.
  - [ ] C. Helical entry for pockets (helical G2/G3, taught to the checker).

## Next

- **GUI pass:** make the GUI a little more attractive and intuitive. It is quite good as it
  is; changes are proposed with screenshots before they're made.

## Later

- **Trochoidal vs adaptive:** decide whether trochoidal should get bigger loops that cover
  the pocket's width, or whether adaptive already does what's wanted.
- **Offset retry** (parked, docs/specs/offset-retry.md, draft): a pocket whose island
  offsets just touch at one exact stepover is refused; retry with a slightly smaller one.
