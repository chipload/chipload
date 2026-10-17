#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Publish the user guide of a release to the gh-pages branch (docs/specs/user-guide.md §5).
# Served at https://chipload.github.io/chipload/. A guide-only fix after a release is tagged
# guide-X.Y.Z-N (docs/releasing.md).
#
#   packaging/docs/publish-book.sh v0.14.0 --yes    build the tag's guide, commit, push
#   packaging/docs/publish-book.sh guide-0.21.0-1 --yes   the same for a guide-only fix
#   packaging/docs/publish-book.sh HEAD --dry-run   every step but the push, from any commit;
#                                                   the result is left in target/gh-pages-dry-run
set -euo pipefail
cd "$(dirname "$0")/../.."

ref=${1:?usage: publish-book.sh vX.Y.Z --yes | REF --dry-run}
mode=${2:-}
case "$mode" in
    --yes)
        # Only released versions are published, so the site never shows unreleased features.
        git rev-parse -q --verify "refs/tags/$ref" > /dev/null || { echo "no tag $ref" >&2; exit 1; }
        ;;
    --dry-run)
        git rev-parse -q --verify "$ref^{commit}" > /dev/null || { echo "no commit $ref" >&2; exit 1; }
        ;;
    *)
        echo "This pushes the guide for $ref to the public gh-pages branch. Add --yes to do it," >&2
        echo "or --dry-run to try every step but the push." >&2
        exit 1
        ;;
esac

work=$(mktemp -d)
cleanup() {
    git worktree remove --force "$work/src" 2> /dev/null || true
    git worktree remove --force "$work/pages" 2> /dev/null || true
    git branch -q -D pages-new 2> /dev/null || true    # the new history's branch, once pushed or not
    rm -rf "$work"
}
trap cleanup EXIT

git worktree add -q --detach "$work/src" "$ref"
# Reuse this checkout's build cache; the book itself goes to the worktree's target/book.
repo=$PWD
(cd "$work/src" && CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}" packaging/docs/build-book.sh)

# The pages branch: the remote's if it exists, else a new one with no history.
git fetch -q origin gh-pages 2> /dev/null || true
if git rev-parse -q --verify origin/gh-pages > /dev/null; then
    git worktree add -q --detach "$work/pages" origin/gh-pages
else
    git worktree add -q --detach "$work/pages"
    (cd "$work/pages" && git checkout -q --orphan pages-new && git rm -rq --cached . && git clean -fdxq)
fi
rsync -a --delete --exclude .git "$work/src/target/book/" "$work/pages/"
touch "$work/pages/.nojekyll"    # serve the files as they are (folders starting with _ too)
(cd "$work/pages" && git add -A && git commit -q -m "User guide for $ref")

if [ "$mode" = "--dry-run" ]; then
    rm -rf target/gh-pages-dry-run
    cp -r "$work/pages" target/gh-pages-dry-run
    rm -rf target/gh-pages-dry-run/.git
    count=$(cd "$work/pages" && git ls-files | wc -l)
    echo "Dry run: the gh-pages commit for $ref holds $count files; nothing pushed."
    echo "Its content: target/gh-pages-dry-run/index.html"
else
    (cd "$work/pages" && git push -q origin HEAD:refs/heads/gh-pages)
    echo "Published the guide for $ref to gh-pages."
fi
