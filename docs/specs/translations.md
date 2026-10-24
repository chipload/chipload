# Spec: translations of the app

Status: **approved 2026-10-24** with the recommended decisions. The user asked for it
("translations for the user interface, in German, French, Spanish, Italian, Swedish") and
chose to start with the app; the user guide stays English for now (§5).

**As built, step 1 (2026-10-24):** `chipload-i18n` and every text of the GUI (about 1,180
in 34 files, `gui-<file>.ftl` per source file), translated into the five languages from
the glossary. The language is chosen under View > Language and *This computer >
Preferences*. Default names of new operations and library entries ("Profile 1", "New
tool") follow the language (decision 9). A decimal comma is read only in the five
languages (decision 6). `packaging/docs/screenshots.sh` pins English for the guide and
takes `SHOTS_LANGUAGE` to check the fit in another language. Still English: what the
library crates write (step 2), the GUI's own two "missing" problems, which are worded like
the library's (`EditJob::missing`, moved with step 2), and startup errors.

## 1. What there is to translate

Measured on 0.23.0:

| Where | What | Size |
|---|---|---|
| `chipload-gui` | menus, buttons, field labels, hovers, dialogs, the Browser, playback, the library window, hints on the canvas | about 730 fixed texts and 250 `format!` texts, 25 000 lines |
| `chipload-job`, `chipload-toolpath`, `chipload-pipeline` | problem messages (`Problem`), planning notes, the import report, the cycle-time and chip-load lines | about 110 problem texts, 330 `format!` in all |
| `chipload-dxf`, `chipload-image`, `chipload-text`, `chipload-check` | import issues, checker findings, errors | a few dozen each |
| `chipload-pipeline` | the setup sheet (HTML) | about 60 texts |
| `chipload-cli`, the user guide, the JSON Schemas | | large; see decision 1 |

The GUI shows texts from every row but the last: a problem under a field and a line in
the report come from the library crates, worded in English with lengths in ` mm`, which
the app turns into inches (`chipload_pipeline::units::text_in`, inch-units.md step 4).

## 2. Decisions

1. **What is translated.** *Recommended:* the app (all of `chipload-gui`'s own text) and
   every message the app shows from the library crates (problems, report, notes, checker
   findings), and the setup sheet, since it goes to the operator at the machine. The
   command line, `--json` output, the MCP tools, the G-code comments, the JSON Schemas and
   the user guide stay English: they are read by scripts and assistants, or are too large
   to keep right in five languages. Built in three steps (§5), so the app is usable in a
   language before the messages are.

2. **How texts are written down: Fluent.** *Recommended:* Project Fluent (`.ftl` files,
   crate `fluent-bundle`, Apache-2.0 OR MIT, with `unic-langid` and `intl_pluralrules`,
   same licences). It handles plurals per language ("1 copy", "2 copies"; Swedish, French
   and Italian differ in which numbers take which form), named arguments and choices, and
   translators and tools (Weblate, Pontoon) know the format. It is pure Rust and compiles
   for the browser (web.md). *Alternative:* our own `key = text` file with `{name}`
   placeholders: no dependency, but plurals would be our own code, and every plural text
   would need its own key per form.

3. **Keys.** Every text gets an id, such as `menu-file-open` or `problem-feed-zero`, in
   `crates/chipload-i18n/locales/<lang>/*.ftl`. English is a language like the others
   (`en`) and the source of truth. *Recommended:* a small new crate `chipload-i18n` at the
   level of `chipload-core` (no other workspace crate, no I/O: the `.ftl` files are
   compiled in with `include_str!`), so both the GUI and the library crates can use it.
   The independent checker may use it too: it shares no code with the generator through
   it. The architecture test gets the crate added.

4. **How the library crates word messages.** A message stops being a finished `String` and
   becomes an id with its arguments (`Message { id, args }`), turned into text where it is
   shown, in the user's language. `Problem::text` (English) stays for the command line and
   `--json`, built from the same id in `en`, so the command line's output doesn't change.
   Numbers with units become typed arguments (`Length(f64)`, `Feed(f64)`) instead of
   ` mm` in the text, so the app writes them in the display units directly and
   `text_in`'s search for ` mm` in a sentence goes away. *Recommended* over translating
   finished English sentences in the app by matching them, which breaks silently the day
   a message is reworded.

5. **The language used.** *Recommended:* the system's language when it is one of ours,
   otherwise English, with a choice in **View > Language** and under *This computer >
   Preferences* (next to the units and the colour theme) (System, English, Deutsch,
   Français, Español, Italiano, Svenska; each named in its own language). Stored in
   `settings.json` as an optional `language` (`"system"` when missing): adding an optional
   field with a safe default needs no version bump. A change applies at once, no restart.
   The system language comes from `sys-locale` (MIT OR Apache-2.0; on Windows and macOS it
   needs the system API, which our `forbid(unsafe_code)` rules out in our own code).

6. **Numbers.** *Recommended:* the decimal point stays everywhere (fields, canvas, report,
   G-code), as in most CAM programs and on the machine's controller, and a field also
   accepts a decimal comma (`6,5` reads as 6.5), since that is what a German, French,
   Spanish, Italian or Swedish keyboard gives on the number pad. Only in those languages:
   in English `1,000` means a thousand, so a comma stays unreadable there, and a text with
   a point and a comma, or two commas, is never guessed at. Lists in fields are
   rare enough that the comma isn't needed as a separator there; where it is (X,Y on the
   command line), the command line is English and unchanged. *Alternative:* a decimal
   comma in display too, per language: more natural to read, but the same number then looks
   different in the app, the G-code and the setup sheet.

7. **Machining words.** One glossary for all five, side by side (`crates/chipload-i18n/GLOSSARY.md`),
   fixed before the texts: profile, pocket, V-carve, climb and conventional milling, step
   down, step over, tabs, onion skin, dogbone, chip load, feed, plunge, ramp, stock, work
   zero, spoilboard, air cut. Terms are taken from the trade's own use in that language
   (tooling makers' catalogues and controller manuals in that language), never from
   another CAM program's translation (clean-room rules). A word with no settled
   translation keeps the English one (*dogbone* is commonly kept in German and Swedish).

8. **Who translates, and how sure we are.** *Recommended:* I write all five from the
   English texts and the glossaries. Swedish you can review yourself. The other four are
   marked in the language list as "(review wanted)" until a native speaker who machines
   has read them, asked for in the README next to the testers. A wrong word in a CNC
   program's interface can mislead (climb versus conventional, inside versus outside), so
   the safety-relevant terms get a second pass: each is checked against how it is used in
   that language's tooling catalogues.

9. **Names that are data.** Names saved in a project (operations' names, layers, parts,
   tools, machine profiles) are the user's and are never translated. Default names given
   to new things ("Profile 1", "Part 2") are written in the user's language when they are
   made, and stay as made. The built-in posts' names (GRBL, LinuxCNC, Mach3) are names.

10. **Fit.** German and French run 20–35 % longer than English. Labels on the left of
    fields, buttons and the Browser's columns are checked in every language at the
    smallest window size (screenshots for each, §6); a label that doesn't fit gets a
    shorter wording, not a smaller font. The fonts already in use (DejaVu, Segoe UI, Arial)
    have every letter the five languages need.

## 3. How it works

- `chipload-i18n` holds one `FluentBundle` per language, built on first use and only read
  after (a table, not state), and `tr(lang, id, args) -> String`. A missing id or argument
  in a language falls back to English; a missing id in English is a bug and a test catches
  it (below), never a panic. Unicode isolation marks around arguments are off: egui would
  draw them as boxes.
- Texts are in `locales/<lang>/<area>.ftl`: `common.ftl`, and one `gui-<file>.ftl` per
  GUI source file, whose ids start with the file's name (`browser-…`, `forms-…`).
  `build.rs` lists the English folder's files, so a new file needs no code change; every
  language folder must hold the same files.
- The GUI keeps the language in its state and, like the display units, in egui's memory
  each frame. Texts are written `t!(ui, "menu-language")` or `t!(self, "app-saved",
  file = name)`: the first argument is anything that knows the language (a `Ui`, the
  `Context`, the app, a `Language`). No global mutable state.
- Lengths in arguments are formatted by the caller's units before they go to Fluent, so
  Fluent never formats a number itself (determinism, and the decimal point of decision 6).

## 4. Tests

- Every id used in the code exists in `en` (a test reads the sources for `t!(` and
  `Message::new(` ids), and every id in `en` is used.
- Every language has every id of `en`, with the same arguments; the test lists the
  missing ones. A release is held back until there are none.
- Every `.ftl` file parses, and every text formats with sample arguments without errors.
- The golden G-code, the setup sheets in English and the command line's output are
  unchanged byte for byte by the move to ids.
- A GUI test opens every window and form in each language (the existing test harness),
  so a text that panics or is missing shows up.

## 5. Steps

1. **The crate and the app's own text.** `chipload-i18n`, Fluent, the language setting,
   every text in `chipload-gui` moved to ids, English only. Then the five translations of
   those texts, with the glossaries. The app is then usable in each language, with
   problems and report lines still in English.
2. **Messages from the library crates** (decision 4): problems, report, notes, checker
   findings, import issues, and their translations. `text_in` is replaced by typed
   length arguments. Planned in detail in §7.
3. **The setup sheet** in the app's language (and `chipload sheet --lang` on the command
   line, the one place the command line takes a language).

The user guide gets a page on choosing the language and on helping with a translation.
The guide itself stays English for now (decided 2026-10-24): 22 000 words that change with
nearly every feature would fall behind in five languages. Its start pages (install, first
part) may be translated later, once the app's features settle.

## 6. Not in this

- Right-to-left scripts and languages needing other fonts (Chinese, Japanese, Arabic).
- Translated user guide, command line, G-code comments.
- Downloading or adding translations at run time: they are compiled in.

## 7. Step 2 in detail: messages from the library crates

Status: **draft 2026-10-24, for review.** Decisions 11–17 are open, each with a
recommendation.

### 7.1 What there is

The library crates write about 450 messages as finished English sentences (`format!`),
and about 50 error variants (`#[error("…")]`):

| Crate | Messages | Where they go |
|---|---|---|
| `chipload-job` | about 240, 97 of them problems | `Problem` (`validate`), migration and load errors |
| `chipload-toolpath` | about 180 | planning notes (`Note`), `job_problems`, offset and plan errors |
| `chipload-pipeline` | about 230 | the report, time and chip-load lines, import report text, `PipelineError`, library and recovery messages, the setup sheet's lines |
| `chipload-post` | about 30 | post config and template errors |
| `chipload-dxf` | about 20 | `Issue` in the `ImportReport`, `ImportError` |
| `chipload-check` | about 20 | `CheckIssue` |
| `chipload-image`, `chipload-text` | 15 errors | a picture or font that can't be read |

The GUI's own two "missing" problems (`EditJob::missing`) go with them, since they sit in
the same list.

They are read by the command line (text), `--json` (`problems[].short/text`, `notes`,
`issues[].message`), the MCP tools, the setup sheet, and the app. In the app, 32 places
pass them through `text_in` to show inches.

### 7.2 Decisions

11. **A message is an id and its arguments.** `chipload_i18n::Message { id, args }`, with
    arguments typed: `Text` (names, which are never translated), `Count` (picks plurals),
    `Length`, `Feed`, `Angle`, `Point` (in mm and degrees), and `Message` (a message inside
    another, such as a problem inside "Operation 3 (Outline): …"). A message is turned
    into text with `render(&message, lang, units)`: lengths and feeds in the display units,
    words in the language. *Alternative:* translate finished English sentences by matching
    them in the app: no change to the library crates, but it breaks silently the day a
    message is reworded (rejected in decision 4).

12. **The English text stays where it is.** `Problem::short` and `text`, `Note::text`,
    `Issue::message` and `CheckIssue::message` keep their English strings, built from the
    message in English and mm when the message is made, and each struct gains the
    `message` itself. So the command line, `--json`, MCP and every test that reads those
    strings are unchanged; only the app switches to rendering `message` in its language and
    units. *Recommended* over removing the strings: scripts and assistants depend on them.

13. **Error types get their text from the same place.** Each error enum gets
    `fn message(&self) -> Message`, and its `Display` renders that in English, replacing
    the `#[error("…")]` text. One wording per error, in the `.ftl` file.

14. **English stays byte for byte, proven by a snapshot.** Before the first message
    moves, a new golden test runs the command line over the corpus (`check-project`,
    `plan`, `inspect` on every project and DXF, `check` on the expected G-code, `--json`
    and text) and freezes its output in `tests/corpus/expected/messages/`. Every later
    commit must leave it unchanged, as the G-code and setup-sheet goldens. Projects that
    are deliberately wrong are added to the corpus first, so most problems appear in it.
    A wording that is plainly wrong in English ("1 moves") may be fixed, said in the
    commit message, with the snapshot's diff.

15. **Lengths keep the format they have today.** A `Length` argument carries how its
    number is written in mm (`0.300 mm` fixed to 3 decimals, `12.5 mm` trimmed), so the
    English is unchanged; in inches it follows the rules `text_in` uses now. The number
    rules move from `chipload_pipeline::units` into `chipload-i18n`, which needs no
    project types to apply them (its own `Units`, mapped from `chipload_job::Units`).
    When every message is moved, `text_in` is deleted.

16. **The checker may use `chipload-i18n`.** It holds texts and number rules, no generator
    code, so the checker can still disagree with the generator (rule 3). The checker's
    texts get a file of their own (`check.ftl`). The architecture test allows it by name.

17. **One `.ftl` file per crate,** `job.ftl`, `toolpath.ftl`, `pipeline.ftl`, `post.ftl`,
    `dxf.ftl`, `check.ftl`, `image.ftl`, `text.ftl`, ids starting with the crate's name
    (`job-feed-zero`). A test, like the GUI's, checks that every id the crates name exists
    and every one is used.

### 7.3 Order

Each its own commit with its translations into the five languages, the snapshot
unchanged and every test passing:

1. `Message`, `render` and the number rules in `chipload-i18n`, with tests that they write
   lengths and feeds exactly as `chipload_pipeline::units` does; the message snapshot test
   and the deliberately wrong corpus projects.
2. **Problems:** `chipload-job`'s `validate`, `chipload-toolpath`'s `job_problems`, the
   units problem in the pipeline, `EditJob::missing`. The most visible: they sit under the
   fields.
3. **Planning notes and the report:** `Note`, the report and its facts, time and chip-load
   lines, the import report text.
4. **Checker findings** (`CheckIssue`).
5. **Errors:** `PipelineError`, `JobError`, `ImportError`, post, image and text errors,
   library and recovery messages.
6. **Done:** `text_in` deleted; the app shows nothing in English but names, units and
   G-code.

### 7.4 Not in step 2

- The setup sheet's own words (step 3); its *Watch out* list follows once its notes are
  messages.
- `--json` stays English. Adding each message's `id` and arguments to it, for scripts
  that want codes instead of sentences, is a small later addition if asked for.

