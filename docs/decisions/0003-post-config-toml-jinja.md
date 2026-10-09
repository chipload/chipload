# 0003: Post-processors as TOML config + sandboxed Jinja2 templates

Status: accepted (2026-10-08); carried over to Rust by ADR 0007

A post-processor is a small versioned TOML file (`schema_version`, name, template,
decimals, file extension, spindle dwell) plus a Jinja2 template.

- TOML is read with the stdlib `tomllib`, so no new dependency. Configs are hand-written
  and read-only for now; nothing writes them yet.
- Templates render in Jinja2's `SandboxedEnvironment` with `StrictUndefined`, because
  user-supplied posts are planned and a typo must fail loudly, not emit an empty word.
- Python does only generic work (number formatting, never `-0.000`, modal compression).
  Machine-specific output stays in the template, per CLAUDE.md rule 5.
