#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the user guide (docs/specs/user-guide.md §5) into target/book/.
# Usage: packaging/docs/build-book.sh [zip]   (zip: also target/chipload-guide-<version>.zip)
set -euo pipefail
cd "$(dirname "$0")/../.."

want=0.5.4
have=$(mdbook --version 2>/dev/null | awk '{print $2}' | tr -d v || true)
if [ "$have" != "$want" ]; then
    echo "The guide is built with mdBook $want (found: ${have:-none})." >&2
    echo "Install it with: cargo install mdbook --version $want --locked" >&2
    exit 1
fi

# The generated reference pages must match the code, and links must point at real pages.
cargo test -q -p chipload-cli --test guide

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
rm -rf target/book
MDBOOK_BOOK__TITLE="Chipload $version user guide" mdbook build docs/book
echo "Built the guide for Chipload $version: target/book/index.html"

if [ "${1:-}" = "zip" ]; then
    zip_path="target/chipload-guide-$version.zip"
    rm -f "$zip_path"
    (
        cd target
        rm -rf "chipload-guide-$version"
        cp -r book "chipload-guide-$version"
        if command -v zip > /dev/null; then
            zip -qr "chipload-guide-$version.zip" "chipload-guide-$version"
        else
            python3 -m zipfile -c "chipload-guide-$version.zip" "chipload-guide-$version"
        fi
        rm -rf "chipload-guide-$version"
    )
    echo "Wrote $zip_path"
fi
