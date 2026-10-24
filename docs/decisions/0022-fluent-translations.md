# 0022: Translations with Fluent, texts compiled in

Status: accepted (2026-10-24). Spec: docs/specs/translations.md.

## Context
The user asked for the app in German, French, Spanish, Italian and Swedish. The app had
about a thousand English texts written in the code, and the messages it shows from the
library crates (problems, report lines, notes) are English sentences too. Plural forms
differ between the languages, and a translator must be able to reorder a sentence's words.

## Decision
- **Fluent** (`fluent-bundle` 0.16, `fluent-syntax`, `unic-langid`; Apache-2.0 OR MIT):
  plurals per language, named arguments, and a file format translators and their tools
  know. Pure Rust; compiles for the browser.
- **A crate of its own, `chipload-i18n`,** beside `chipload-core`: no I/O, no other
  workspace crate, so the GUI now and the library crates later (step 2, messages) can use
  it. The `.ftl` files are compiled in (`include_str!`, listed by `build.rs`); nothing is
  read at run time.
- **English is the source** and a language like the others. A text missing in another
  language falls back to English; tests require every language to have every English text
  with the same arguments, and every `t!` id in the GUI to exist.
- **The language** follows the system (`sys-locale`, MIT OR Apache-2.0: on Windows and
  macOS it needs the system API, which `forbid(unsafe_code)` keeps out of our code), with
  Settings > Language to choose; kept in `settings.json` as an optional `language`.
- **Numbers keep the decimal point**; the caller formats them, Fluent never does.
- **Not translated:** the command line, `--json`, MCP, G-code, schemas and the user guide.

## Consequences
- A new text in the app is a `t!` call and a line in every language's `.ftl` file; the
  tests list what is missing.
- Four languages are machine-made until native speakers review them; the language menu
  says so (decision 8).
