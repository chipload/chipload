# 0007: Rust post-processor with minijinja, shared templates

Status: accepted (2026-10-09)

The Rust port renders G-code in `chipload-post` with the same data-driven design as Python
(CLAUDE.md rule 5): a versioned TOML config plus a Jinja template, and only generic work
(number formatting, modal compression) in code.

## Libraries
- `minijinja` 3.0 (Apache-2.0): Jinja2-compatible templates by the author of Jinja2. Templates
  only see the context they are given, so they are sandboxed by design, like Python's
  `SandboxedEnvironment`. Configured identically: trim_blocks, lstrip_blocks,
  keep_trailing_newline, strict undefined, no auto-escaping.
- `toml` 1.x (MIT OR Apache-2.0): post configs.
- `sha2` 0.10 (MIT OR Apache-2.0, RustCrypto): `source.sha256` check in `chipload run`.

## One copy of every post
During the port the built-in configs and templates stayed in the Python package and the Rust
crate compiled them in with `include_str!`, so both implementations rendered from the same
files; a conformance test (removed with Python, ADR 0009) required byte-identical output for
the same toolpath, on the golden files and on Hypothesis-generated toolpaths. With Python gone
the files live in `crates/chipload-post/posts/` and `templates/`.

## Number formatting
Rust's `{:.N}` and Python's `f"{v:.Nf}"` both round the exact binary value with ties to
even; checked on ties such as 0.125 and 2.5 and covered by the random-toolpath test.
