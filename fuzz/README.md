# Fuzzing the DXF importer and the image tracer

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

## Pictures

`fuzz_targets/image_trace.rs` does the same for pictures (docs/specs/image-trace.md §8):
any bytes are decoded, and a picture that decodes is traced. The decoder must never panic
outside the third-party code, and every traced loop must have finite points. Seed it with
the test pictures:

```sh
mkdir -p corpus/image_trace && cp ../tests/corpus/images/*.png corpus/image_trace/
cargo +nightly fuzz run image_trace corpus/image_trace -- -max_total_time=900 -timeout=10 -max_len=4096
```

First run (2026-10-13): 3.1 million inputs in 5 minutes, nothing found.

## Found so far

DXF:
- `huge_coordinate.dxf` (2026-10-13): a 127-digit coordinate overflowed the contour
  joining's grid. Entities past `MAX_COORDINATE_MM` (1 km) are now skipped and reported.

`corpus/`, `artifacts/` and `target/` are local and not committed.
