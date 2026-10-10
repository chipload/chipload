# 0012: Build `cavalier_contours` without debug assertions in dev and test builds

Status: accepted (2026-10-10)

## Context
The pocket property tests found jagged but valid islands (no self-intersection, no folds;
see docs/specs/folded-contours.md) where `cavalier_contours` 0.9.0 trips one of its own
`debug_assert!`s inside `Shape::parallel_offset`. Chipload turns the panic into
"Pocket offset failed inside the geometry library". Release builds don't have those assertions:
the same input offsets fine there, and Chipload's own verification (no gouge, everything
inside the region; ADR 0006) passes on the result.

So debug and test builds refused input that release builds, which users run, handle. The
tests were testing a different library than the one we ship.

## Decision
`[profile.dev.package.cavalier_contours] debug-assertions = false` in the workspace
`Cargo.toml`. Only that crate is affected: Chipload's own crates keep their debug
assertions and overflow checks.

## Consequences
- Tests exercise the library code that ships, including its behaviour on such input.
- We lose cavalier's internal consistency checks as an early warning. That's acceptable
  because nothing it returns is trusted: every offset is verified after the fact (ADR 0006),
  and the property tests check the outcome (no gouge, coverage, material left).
- A panic that isn't a debug assertion still becomes an error via `catch_unwind`, as before.
