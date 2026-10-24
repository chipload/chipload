# Deliberately wrong projects and G-code

For the message snapshot (`crates/chipload-cli/tests/messages.rs`,
docs/specs/translations.md decision 14): what the command line says about these must stay
the same while messages move into `chipload-i18n`. Frozen like the other corpus files:
add new ones, don't edit these.

Made 2026-10-24 from corpus projects (dogbones, holes, finish_skin, follow_path,
vcarve_letters, surface, rest_passes, straight_tabs, two_parts) saved at schema 29:

- `*_zero`, `*_negative`, `*_huge`: every decimal number set to 0, to its negative minus
  one, or to 10 000 000 (ids, counts and the schema version kept).
- `*_refs`: every operation's tool and every shape's DXF set to 99, every layer selector
  to one that doesn't exist.
- `*_big_tool`: every tool 60 mm across.
- `*_deep`: every operation cut 5 mm below the stock, 100 mm per pass, safe Z 0.5 mm.
- `bad.nc`, `bad_mm.nc`: G-code that breaks the checker's rules line by line (the second
  with G21 first).
