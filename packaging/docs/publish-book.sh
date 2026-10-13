#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Publish the user guide of a release to the gh-pages branch (docs/specs/user-guide.md §5).
# Not used while the repository is private: GitHub Pages needs a public repository.
# Usage: packaging/docs/publish-book.sh v0.14.0 --yes
set -euo pipefail
cd "$(dirname "$0")/../.."

tag=${1:?usage: publish-book.sh vX.Y.Z --yes}
if [ "${2:-}" != "--yes" ]; then
    echo "This pushes the guide for $tag to the public gh-pages branch. Add --yes to do it." >&2
    exit 1
fi
git rev-parse -q --verify "refs/tags/$tag" > /dev/null || { echo "no tag $tag" >&2; exit 1; }

work=$(mktemp -d)
trap 'git worktree remove --force "$work/src" 2>/dev/null || true; rm -rf "$work"' EXIT
# Build from the release itself, so the site shows only released features.
git worktree add --detach "$work/src" "$tag"
(cd "$work/src" && packaging/docs/build-book.sh)

git fetch -q origin gh-pages 2>/dev/null || true
if git rev-parse -q --verify origin/gh-pages > /dev/null; then
    git worktree add --detach "$work/pages" origin/gh-pages
else
    git worktree add --orphan -b gh-pages "$work/pages"
fi
rsync -a --delete --exclude .git "$work/src/target/book/" "$work/pages/"
touch "$work/pages/.nojekyll"
(cd "$work/pages" && git add -A && git commit -q -m "User guide for $tag" && git push origin HEAD:gh-pages)
git worktree remove --force "$work/pages"
echo "Published the guide for $tag to gh-pages."
