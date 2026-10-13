# Fuzzing the DXF importer

DXF files are the input most likely to be hostile (CLAUDE.md: "treat all input as
hostile"). `fuzz_targets/dxf_import.rs` feeds random bytes through the importer as a file
would go. It checks that:
- it never panics outside the third-party parser (whose panics the importer turns into
  errors);
- it never hangs;
- every imported coordinate is finite.

Run it before a release, or after changing the importer. It needs a nightly toolchain and
cargo-fuzz, neither of which the project itself uses:

```sh
rustup toolchain install nightly --profile minimal
cargo +nightly install cargo-fuzz --locked
cd fuzz
mkdir -p corpus/dxf_import && cp ../tests/corpus/dxf/*.dxf ../tests/corpus/dxf/fuzz/*.dxf corpus/dxf_import/
cargo +nightly fuzz run dxf_import corpus/dxf_import -- -max_total_time=900 -timeout=10 -max_len=200000
```

When it finds something, the input is saved in `artifacts/dxf_import/`. To find out what
it broke, give it to the real command line:

```sh
cp artifacts/dxf_import/crash-* /tmp/crash.dxf
RUST_BACKTRACE=1 cargo run -p chipload-cli -- inspect /tmp/crash.dxf --units mm
```

Then follow CLAUDE.md: add the file to `tests/corpus/dxf/fuzz/` with a failing test first,
then fix.

Found so far:
- `huge_coordinate.dxf` (2026-10-13): a 127-digit coordinate overflowed the contour
  joining's grid. Entities past `MAX_COORDINATE_MM` (1 km) are now skipped and reported.

`corpus/`, `artifacts/` and `target/` are local and not committed.
