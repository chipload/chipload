# Releasing

Releases are built by GitHub Actions (`.github/workflows/release.yml`, ADR 0010) when a
version tag is pushed. The workflow creates a **draft** release; nothing is public until you
publish it.

1. Bump `version` in the workspace `Cargo.toml` (e.g. `0.2.0`) **and** the internal
   dependency pins (`chipload-* = { version = "0.2.0", path = … }` in `crates/*/Cargo.toml`),
   then run `cargo build` so `Cargo.lock` follows.
2. Regenerate the notices. Always: they list Chipload's own crates with their version, so
   every version bump changes them, and CI fails if they're stale:

       cargo about generate --fail about.hbs > THIRD-PARTY-LICENSES.txt

3. Commit, then tag and push the tag:

       git tag -a v0.2.0 -m "Chipload 0.2.0"
       git push origin v0.2.0

4. The workflow checks that the tag matches the workspace version, runs the tests on Linux
   and Windows (x86_64 and ARM64) and macOS, builds the files and the user guide, and creates
   a draft release with them and `SHA256SUMS.txt`. To try the builds without a release, run
   it by hand: *Actions > Release > Run workflow* (the files are left as run artifacts).
5. Download and try the AppImages, the Windows `.exe` files and the macOS app, edit the notes,
   then **Publish**.
6. The user guide (docs/specs/user-guide.md §5): renew its screenshots **before tagging**
   (needs a display) and commit them if they changed; the workflow builds the zip from the
   tag:

       packaging/docs/screenshots.sh         # docs/book/src/images/*.png

   Without the workflow, build and attach it by hand:

       packaging/docs/build-book.sh zip      # target/chipload-guide-X.Y.Z.zip
       gh release upload vX.Y.Z target/chipload-guide-X.Y.Z.zip

   Publish it at https://chipload.github.io/chipload/ (the gh-pages branch):
   `packaging/docs/publish-book.sh vX.Y.Z --yes`.

   A fix to the guide alone after a release (no change under `crates/` since the tag): tag
   the commit `guide-X.Y.Z-N` (not `v…`, which would start the release workflow), push the
   tag and publish that: `packaging/docs/publish-book.sh guide-X.Y.Z-N --yes`. While the
   release is still a draft, also rebuild the guide zip and upload it again with
   `gh release upload vX.Y.Z target/chipload-guide-X.Y.Z.zip --clobber`.

## Files

| File | What |
|---|---|
| `chipload-gui-X.Y.Z-linux-x86_64.AppImage` | Linux GUI; glibc 2.35+ (Ubuntu 22.04, Debian 12, Fedora 36 or newer) |
| `chipload-gui-X.Y.Z-linux-aarch64.AppImage` | the same for ARM64: Raspberry Pi 4 and 5 with a 64-bit OS, ARM boards and laptops |
| `chipload-gui-X.Y.Z-windows-x86_64.exe` | Windows GUI, single file (static C runtime), not code-signed |
| `chipload-gui-X.Y.Z-windows-aarch64.exe` | the same for Windows on ARM (Snapdragon laptops) |
| `chipload-X.Y.Z-linux-x86_64.tar.gz`, `…-linux-aarch64.tar.gz` | Linux command line, with LICENSE and the third-party notices |
| `chipload-X.Y.Z-windows-x86_64.exe`, `…-windows-aarch64.exe` | Windows command line |
| `chipload-gui-X.Y.Z-macos-universal.dmg` | macOS GUI (`Chipload.app`, Apple silicon and Intel, macOS 11+), signed ad hoc, not notarised |
| `chipload-X.Y.Z-macos-universal.tar.gz` | macOS command line, with LICENSE and the third-party notices |
| `SHA256SUMS.txt` | checksums of the above |
| `chipload-guide-X.Y.Z.zip` | the user guide (HTML; open `index.html`) |

GitHub attaches the source archives of the tag, which with `LICENSE` covers the GPL's
corresponding-source requirement. Every binary also carries the licence texts (GUI:
Help > About and licences; CLI: `chipload licenses`).

## Building locally

    cargo build --release -p chipload-gui -p chipload-cli
    packaging/linux/build-appimage.sh 0.2.0 dist        # Linux AppImage in dist/

On Windows, `cargo build --release` gives the single-file executables in `target/release/`
(`.cargo/config.toml` links the C runtime statically).

## Without GitHub Actions

When the workflow can't run (no Actions minutes), build the Linux files locally in an
Ubuntu 22.04 container, with the same steps and the same glibc:

    packaging/linux/build-release-local.sh 0.13.0 dist/local

Then create the draft with `gh release create v0.13.0 dist/local/chipload-gui-*.AppImage
dist/local/chipload-*.tar.gz dist/local/SHA256SUMS.txt --draft --verify-tag --title
"Chipload 0.13.0" --notes-file notes.md` (the `gh` token needs Contents read/write).

The Windows files are built on the Windows side of the same machine: Rust (rustup, the
default `msvc` toolchain), the Visual Studio Build Tools ("Desktop development with C++")
and Git, with a checkout on a Windows drive (e.g. `C:\code\chipload`, not over `\\wsl$`).
From WSL, in `/mnt/c/code/chipload`:

    cmd.exe /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && git fetch --tags && git checkout v0.13.0 && set PROPTEST_RNG_SEED=20261013&& cargo test --workspace --locked --no-fail-fast && cargo build --release --locked -p chipload-gui -p chipload-cli"

`rust-toolchain.toml` makes rustup install the pinned Rust on the first run. Then:

1. Copy `target/release/chipload-gui.exe` to `chipload-gui-0.13.0-windows-x86_64.exe` and
   `target/release/chipload.exe` to `chipload-0.13.0-windows-x86_64.exe`.
2. Check that neither needs the VC++ runtime: `strings -a FILE | grep -iE
   '^(vcruntime|msvcp|api-ms-win-crt)'` prints nothing.
3. Add their checksums to the release's `SHA256SUMS.txt` (`gh release download v0.13.0 -p
   SHA256SUMS.txt`, then `sha256sum *.exe >> SHA256SUMS.txt`).
4. `gh release upload v0.13.0 chipload-gui-0.13.0-windows-x86_64.exe
   chipload-0.13.0-windows-x86_64.exe SHA256SUMS.txt --clobber`, and list the Windows files
   in the release notes.
5. `git switch main` in the Windows checkout, so it isn't left on the tag.

### macOS

There is no Mac here, so without Actions a release has no macOS files; say so in its notes.
On a Mac with Rust, Xcode's command line tools and Homebrew's `librsvg`, the workflow's
steps are:

    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    export MACOSX_DEPLOYMENT_TARGET=11.0
    packaging/test-release.sh
    cargo build --release --locked --target aarch64-apple-darwin -p chipload-gui -p chipload-cli
    cargo build --release --locked --target x86_64-apple-darwin -p chipload-gui -p chipload-cli
    packaging/macos/build-macos.sh 0.18.0 dist

Then add both files' checksums to `SHA256SUMS.txt` and upload them as for Windows.

## When the repository goes public: the user guide's website

The guide is ready to publish (docs/specs/user-guide.md §5); `packaging/docs/publish-book.sh
HEAD --dry-run` checks every step but the push. Then:

1. Publish a release's guide: `packaging/docs/publish-book.sh vX.Y.Z --yes`. Only tags are
   published, so the first release after the guide (0.14.0 or later) is the first one with it.
2. On GitHub: *Settings > Pages > Build and deployment*: source *Deploy from a branch*,
   branch `gh-pages`, folder `/ (root)`. The site is then at
   `https://chipload.github.io/chipload/`.
3. Point the app at it: set `USER_GUIDE_URL` in `crates/chipload-gui/src/app/dialogs.rs`,
   and replace the "comes with each release" text in the guide's install page if wanted.
4. From then on, run step 1 for every release, after step 6 above.
