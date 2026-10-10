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
   and Windows, builds the files, and creates a draft release with them and `SHA256SUMS.txt`.
5. Download and try the AppImage and the Windows `.exe`, edit the notes, then **Publish**.

## Files

| File | What |
|---|---|
| `Chipload-X.Y.Z-x86_64.AppImage` | Linux GUI; glibc 2.35+ (Ubuntu 22.04, Debian 12, Fedora 36 or newer) |
| `chipload-gui-X.Y.Z-windows-x86_64.exe` | Windows GUI, single file (static C runtime), not code-signed |
| `chipload-X.Y.Z-linux-x86_64.tar.gz` | Linux command line, with LICENSE and the third-party notices |
| `chipload-X.Y.Z-windows-x86_64.exe` | Windows command line |
| `SHA256SUMS.txt` | checksums of the above |

GitHub attaches the source archives of the tag, which with `LICENSE` covers the GPL's
corresponding-source requirement. Every binary also carries the licence texts (GUI:
Help > About and licences; CLI: `chipload licenses`).

## Building locally

    cargo build --release -p chipload-gui -p chipload-cli
    packaging/linux/build-appimage.sh 0.2.0 dist        # Linux AppImage in dist/

On Windows, `cargo build --release` gives the single-file executables in `target/release/`
(`.cargo/config.toml` links the C runtime statically).
