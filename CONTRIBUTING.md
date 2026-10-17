# Contributing to Chipload

Thanks for helping. Chipload drives real machines, so correctness and safety come first.

## License and sign-off

Chipload is licensed under GPL-3.0-or-later. By contributing you agree that your contribution
is licensed under the same terms. Every commit must carry a Developer Certificate of Origin
sign-off (https://developercertificate.org):

    git commit -s

which adds `Signed-off-by: Your Name <you@example.com>`.

## Your own work only (clean room)

Chipload is a clean-room implementation in Rust: every line is written here. Your
contribution must be your own work, built only on publicly available, general knowledge.

- **Allowed:**
  - textbooks, published papers and standards (RS-274/NIST G-code);
  - controller and tooling makers' public documentation (GRBL, LinuxCNC, Mach3/4 manuals;
    feeds and chip loads);
  - well-known algorithms, and first principles.

  Name the source in the spec when an algorithm comes from one.
- **Not allowed:** code copied, ported, translated or paraphrased from another CAM program or
  toolpath library, open source or proprietary, whatever its licence (GPL-compatible
  included). Nor their post-processor templates, help text, UI wording, icons or sample
  files, nor anything reverse engineered from their binaries or output.
- **Other programs** may be named for what users expect of a feature ("CamBam calls this
  Parts"), not for how they do it inside.
- **Libraries** are used as dependencies through their APIs, never copied into the
  repository.

Your DCO sign-off (above) certifies this. If you're unsure where an idea or a piece of code
comes from, ask in the pull request before writing it. The full rule is in `CLAUDE.md`.

## Before you open a pull request

- Read `docs/architecture.md` for how the crates fit together.
- Read `CLAUDE.md`: architecture rules, conventions and safety rules apply to everyone.
- New toolpath operations or non-trivial algorithms start with a spec in `docs/specs/`.
- Run the checks:

      cargo test --workspace
      cargo clippy --workspace --all-targets -- -D warnings
      cargo fmt --all --check
      cargo deny check licenses bans sources

- Changing golden G-code needs a note in the commit message explaining why.
- New dependencies: state the license (must be GPL-3.0-compatible, see `CLAUDE.md`) and why.

## Testing on a real machine

Reports from real machines are the most useful help right now: see *Testing on your
machine* in `README.md` for what to try and what to send, also when it worked.

## Reporting a bad toolpath

Attach the DXF (anonymised if needed), the job file, the generated G-code and what went wrong.
With your permission the drawing is added to the test corpus with a failing test first.
