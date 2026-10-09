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
The built-in configs and templates stay in `src/chipload/core/post/` and the Rust crate
compiles them in with `include_str!`. Both implementations render from the same files, and
`tests/conformance/test_post_parity.py` requires byte-identical output for the same toolpath
(golden files and Hypothesis-generated toolpaths). When the Python version is removed
(ADR 0004), the files move next to the Rust crate.

## Number formatting
Rust's `{:.N}` and Python's `f"{v:.Nf}"` both round the exact binary value with ties to
even; checked on ties such as 0.125 and 2.5 and covered by the random-toolpath test.
