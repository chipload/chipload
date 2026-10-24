# Languages

The app speaks English, German (Deutsch), French (Français), Spanish (Español), Italian
(Italiano) and Swedish (Svenska).

## Choosing the language

At the start the app follows your computer's language, and English when it isn't one of
these. To choose another, use *View > Language* or *This computer > Preferences* in the
Browser. The change is immediate, and the app keeps it for next time.

Problems, the report, the checker's findings, error messages and the
[setup sheet](checking/sheet.md) follow the language too. What stays in English, whatever
the language:
- The G-code and its comments.
- The command line, `--json` output and the [AI assistant tools](assistants.md) (a setup
  sheet written by `chipload sheet --lang` or `chipload run --sheet-lang` excepted).
- The issues found when a DXF was first read, which the project keeps as they were
  written, and what other programs say (the file system, a picture that can't be read).
- This guide.

Names you give things (parts, operations, tools, machines, layers) are yours and are never
translated. Numbers keep the decimal point in every language. In German, French, Spanish, Italian
and Swedish a field also takes a decimal comma (`6,5`); in English a comma isn't read, since
`1,000` would mean a thousand.

## Helping with a translation

German, French, Spanish and Italian are marked *(review wanted)* in the language menu: they
haven't yet been read by someone who speaks the language and runs a CNC machine. A wrong
word in a CAM program can mislead (climb or conventional, inside or outside), so your
corrections matter.

The texts are plain files in the source, one folder per language:
`crates/chipload-i18n/locales/<language>/*.ftl`. The machining terms used all through
them are in `crates/chipload-i18n/GLOSSARY.md`. [Open an
issue](https://github.com/chipload/chipload/issues) with the wrong text and a better one,
or send a pull request that changes the file.
