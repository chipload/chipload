# Contributing to Chipload

Thanks for helping. Chipload drives real machines, so correctness and safety come first.

## License and sign-off

Chipload is licensed under GPL-3.0-or-later. By contributing you agree that your contribution
is licensed under the same terms. Every commit must carry a Developer Certificate of Origin
sign-off (https://developercertificate.org):

    git commit -s

which adds `Signed-off-by: Your Name <you@example.com>`.

## Before you open a pull request

- Read `CLAUDE.md`: architecture rules, conventions and safety rules apply to everyone.
- New toolpath operations or non-trivial algorithms start with a spec in `docs/specs/`.
- Run the checks:

      cargo test --workspace
      cargo clippy --workspace --all-targets -- -D warnings
      cargo fmt --all --check
      cargo deny check licenses bans sources

- Changing golden G-code needs a note in the commit message explaining why.
- New dependencies: state the license (must be GPL-3.0-compatible, see `CLAUDE.md`) and why.

## Reporting a bad toolpath

Attach the DXF (anonymised if needed), the job file, the generated G-code and what went wrong.
With your permission the drawing is added to the test corpus with a failing test first.
