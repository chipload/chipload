#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# The tests as the release workflow runs them (docs/releasing.md). The property tests (proptest)
# draw random cases; here they use a fixed seed, so a release build can't fail on a case no
# one has seen, and any failure can be replayed exactly with the same seed. Everyday runs
# (`cargo test`, CI) keep drawing new cases, and failures they find are saved in
# crates/*/tests/*.proptest-regressions, which are committed and always run first.
set -eu
PROPTEST_RNG_SEED=${PROPTEST_RNG_SEED:-20261013}
export PROPTEST_RNG_SEED
echo "Property tests with PROPTEST_RNG_SEED=$PROPTEST_RNG_SEED"
# As release builds (--release): the release build after this reuses every compiled crate
# instead of compiling them all a second time, and the tests run faster. The debug checks
# are off here; CI runs the tests as debug builds on Linux and Windows on every push.
exec cargo test --release --workspace --locked --no-fail-fast "$@"
