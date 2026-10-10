# Spec: retrying an offset that hit a numerical coincidence

Status: **draft 2026-10-10**, waiting for review (questions in §5).

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

## 3. Profiles

Profile offsets use the same library and verification. No failing case is known yet. The
same retry (r + kε) can be applied there at no cost.

## 4. Tests

- The ignored test becomes a normal test: the pocket is cleared and every ring keeps the
  tool radius from walls and islands.
- A unit test that the retry is used only after a failure (a pocket that works is
  byte-identical with and without the retry code path).
- Golden G-code unchanged; property tests unchanged.

## 5. Questions for review

1. **Retry** as in §2: stepover −ε, −2ε, −3ε with ε = 1e-6 mm, never a larger stepover.
   *Recommendation: yes.*
2. **Wall pass** too (offset r + kε) when the first ring is the one that fails.
   *Recommendation: yes.*
3. **Tell the user?** A note like "rings rebuilt after a numerical problem" would worry more
   than help, since the G-code is identical to 3 decimals. *Recommendation: no note.* It's not
   a machining choice, only a way past a library limitation; the spec documents it.
4. **Profiles** get the same retry now, without a known failing case. *Recommendation: yes*,
   so a profile doesn't hit the same wall later.
