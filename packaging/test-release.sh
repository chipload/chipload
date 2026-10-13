#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# The tests as release builds run them (docs/releasing.md). The property tests (proptest)
# draw random cases; here they use a fixed seed, so a release build can't fail on a case no
# one has seen, and any failure can be replayed exactly with the same seed. Everyday runs
# (`cargo test`, CI) keep drawing new cases, and failures they find are saved in
# crates/*/tests/*.proptest-regressions, which are committed and always run first.
set -eu
PROPTEST_RNG_SEED=${PROPTEST_RNG_SEED:-20261013}
export PROPTEST_RNG_SEED
echo "Property tests with PROPTEST_RNG_SEED=$PROPTEST_RNG_SEED"
exec cargo test --workspace --locked --no-fail-fast "$@"
