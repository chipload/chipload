# Spec: retrying an offset that hit a numerical coincidence

Status: **reviewed 2026-10-11**, all four questions in §5 as recommended.

## 1. The problem

The pocket property test found a valid pocket (four contours, tool radius 4.865 mm, saved as
the ignored test `a_pocket_whose_island_offsets_just_touch_can_be_cleared`) that Chipload
refuses: "pocket toolpath failed verification (too close to a wall or island)".

What was measured (2026-10-10):
- The outline, the islands and the tool are fine on their own; it fails only with islands
  #2 and #4 together, and only at **exactly** one stepover. With the stepover
  1e-6 mm smaller or larger, the pocket is cleared in 8 rings that pass verification.
- So at one ring level the offsets of the two islands meet almost exactly tangentially.
  cavalier_contours trips its own assertion there in debug builds, and in release builds it
  returns rings that come too close to an island. Chipload's verification catches that
  (no G-code), so it's safe, but a valid drawing is refused at random-looking settings.
- First suspected: a run of 0.005 mm segments in the outline. Merging those changed nothing,
  so the drawing is not the cause and doesn't need repairing.

Any drawing can hit this, by coincidence of geometry, tool and stepover. Rare, but a user
can't fix it except by changing the stepover by an invisible amount.

## 2. Fix: retry with a negligibly smaller offset step

When the rings of a pocket fail verification:

1. Build the rings again with the stepover reduced by ε, then 2ε, then 3ε, where
   ε = `OFFSET_RETRY_NUDGE_MM` = 1e-6 mm (new constant in `chipload_core::tolerances`).
2. The first attempt whose rings pass the **unchanged** verification is used.
3. If all three fail, the error is the same as today.

Why this is safe:
- A smaller stepover only makes rings overlap more: never more material left, never closer
  to a wall. Verification and the uncut-material check run on the result as always.
- 1e-6 mm is a thousand times below the chaining tolerance and invisible in G-code written
  with 3 decimals; output for every drawing that works today is unchanged (the retry only
  runs after a failure), so golden files don't change.
- Deterministic: the same input always takes the same attempts.

For the first ring (the wall pass, offset by the tool radius r), the same coincidence would
need r itself to move: that attempt offsets by r + ε instead (1e-6 mm farther from the wall).
Once a wall ring has failed, every later attempt moves it (r + kε, stepover − kε).

What counts as a failure worth retrying: the geometry library failing (a panic or an
inconsistent result such as offsets that aren't nested), or the rings failing verification.
Errors that a nudge can't change (the tool doesn't fit, too many rings) are returned at once.
If a retry ends in such an error, the first attempt's error is returned, as today.

The rings are verified (without links or leads) as soon as they are built, so a failure is
retried before anything is planned on top of them; the full verification still runs at the
end as before. Each attempt records its arcs separately and only the used attempt's arcs are
kept.

### Adaptive and trochoidal pockets

They shrink the pocket by the same r for their wall pass, so the same coincidence stops them
(found by the property tests on 2026-10-11, once ring pockets got past it). They take the
same retry with the wall moved on every retry (r + kε; trochoidal backbones also step kε
closer). They have no rings at one stepover to nudge on their own, and their errors (from
the geometry library or their own checks) can't be told apart, so each one is retried; an
error a nudge can't change just comes back as the first attempt's.

## 3. Profiles

Profile offsets use the same library and verification. No failing case is known yet. The
same retry (r + kε) is applied there at no cost. A profile ring checks its distance both
ways (at least r − tolerance, at most r + slack), so a nudged attempt is checked against
r + kε, the distance it was asked for: the no-gouge bound only gets stricter.

## 4. Tests

- The ignored test becomes a normal test: the pocket is cleared and every ring keeps the
  tool radius from walls and islands.
- Unit tests of the retry: no retry after a success, attempts in order with the right
  nudges, the wall moved only after a wall failure, the first error kept, no retry after an
  error a nudge can't change.
- Golden G-code unchanged shows that working pockets and profiles are byte-identical.
- A pocket whose wall pass hits the coincidence is cleared by all three strategies.
- A profile attempt made to fail is retried one nudge farther out and verified there.
- Golden G-code unchanged; property tests unchanged.

## 5. Questions for review (answered 2026-10-11: 1 yes, 2 yes, 3 no note, 4 yes)

1. **Retry** as in §2: stepover −ε, −2ε, −3ε with ε = 1e-6 mm, never a larger stepover.
   *Recommendation: yes.*
2. **Wall pass** too (offset r + kε) when the first ring is the one that fails.
   *Recommendation: yes.*
3. **Tell the user?** A note like "rings rebuilt after a numerical problem" would worry more
   than help, since the G-code is identical to 3 decimals. *Recommendation: no note.* It's not
   a machining choice, only a way past a library limitation; the spec documents it.
4. **Profiles** get the same retry now, without a known failing case. *Recommendation: yes*,
   so a profile doesn't hit the same wall later.

## 6. The tight slice tolerance (2026-10-13)

The rest machining property test found pockets that the nudges in §2 don't save (rest
areas, which are unions of circles, and once a plain pocket). cavalier_contours' shape
offset keeps a slice of the raw offset when it lies at least `offset_dist_eps` short of
the offset distance; the default 1e-4 kept, here, a joining arc that runs 0.028 mm too
close to a corner. That is caught by the verification, so it's a refusal, never a gouge.
In other cases, the same thing left an island's offset outside every area.

With `offset_dist_eps` = 1e-6 (`OFFSET_TIGHT_SLICE_EPS_MM`), those offsets are exact. Not as
the default: on another pocket it lost a whole area (42 × 22 mm left uncut), so it is a
retry only:

- **Pocket rings:** after the nudged attempts of §2 have all failed, one more attempt
  without a nudge and with the tight tolerance (`nudged_or_tight`). Profiles keep `nudged`:
  their single-loop offsets don't use the shape offset.
- **Any pocket shape offset** that ends in an error (an island across an area, a library
  panic) is computed once more with the tight tolerance before the error is returned.
- **An island's offset that neither crosses nor holds any area** is dropped: it takes
  nothing away from them.

As before, output only changes where Chipload used to refuse: the golden files don't change.
Two rarer cases (about 1 in 4000 random pockets) were still refused; §7 fixes them.

## 7. Wider nudges (2026-10-13)

The two rarer refusals fail at one exact distance **and the next few nanometres**: the shape
offset put an island's offset across an area at the small tool's radius and at +1, +2 and +3
nm (tight tolerance or not), but not at +10 nm or at −1 nm. A ring set failed verification
the same way.

- **The ladder** (`retry::NUDGES`): after the three 1 nm nudges, three of 10 nm
  (`OFFSET_RETRY_WIDE_NUDGE_MM`), then the tight attempt as in §6. Earlier attempts are
  unchanged, so a pocket or profile that used to succeed gets exactly the same paths.
- **Rest machining's own shrinks** (where the larger tool's centre can go, and whether a rest
  area fits the smaller tool) weren't retried at all. On an error, they're now shrunk
  farther by each nudge in turn. Farther is the safe side: the larger tool is taken to reach
  up to 30 nm less, so the rest pocket cuts a little more. That's far below the 0.01 mm wall
  gap that material left keeps, so it never shows as material.
- **The cost:** a wall nudged by 30 nm leaves up to 30 nm more on that wall, against the
  100 nm (`OFFSET_FAR_SLACK_MM`) a profile may already leave.

Both minimal cases are tests (`…_retried_farther` in `tests/pocket.rs`).
