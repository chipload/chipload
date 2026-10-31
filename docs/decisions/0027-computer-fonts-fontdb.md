# 0027: Texts in the computer's fonts, found with fontdb

Status: accepted (2026-10-31). Spec: docs/specs/system-fonts.md (agreed 2026-10-31).

## Context
Texts could only use the 17 built-in fonts (ADR 0020, 0021). The user asked for the
computer's fonts too. A text's letters are stored in the project as shapes, so a font is
only needed to make them again, never to cut.

## Decision
- **A project names its font, nothing more:** no font file goes into it (fonts are
  licensed to the computer they are installed on). No format change: older versions
  already keep the letters of a text whose font they don't know.
- **`chipload-text` stays without I/O:** it makes letters from a font file's bytes
  (`letters_in`) and names a face (`face_name`, its full name from the name table).
- **`chipload-pipeline::fonts`** finds fonts: built in first, then the settings folder's
  `fonts` folder, then the installed ones, through a `FontSource`. Looked for once, the
  first time a non-built-in font is needed.
- **`fontdb` 0.24** (MIT) lists the installed fonts on Windows, macOS and Linux (its
  `fontconfig` feature reads the Linux configuration without the C library). Without its
  default `memmap` feature: memory-mapping is `unsafe` and a file changed underneath could
  crash the app; the files are read instead. A target dependency of the pipeline, not in
  the browser build, which has the built-in fonts only (`BuiltinOnly`).
- New crates in the lock file, all MIT or MIT/Apache-2.0/Zlib: fontdb, fontconfig-parser,
  roxmltree 0.20 (a second version beside usvg's), tinyvec.

## Consequences
- A text whose font isn't on the computer keeps its letters and cuts; changing its letters
  is refused with a note, moving and turning move the stored letters.
- The pipeline does file I/O for fonts, as it already does for the library and posts.
