# Spec: the report's details folded away

Status: approved (2026-11-07; decisions 1–6 as recommended, the user: "do these and then
we release 0.34"); built.

Builds on docs/specs/look-and-feel.md §6 (the report, easy to scan). Guide:
docs/book/src/window.md (the Report) and docs/book/src/checking/time.md.

## 1. Why

The user (2026-11-07): "in general the report panel is a bit information overloaded so to
say". After a generate the Report showed, under its headline and warnings, the time table,
a card of notes (the cut outline, the whole job's lengths, what the estimate leaves out,
operation facts, checker notes, the work zero) and the drawing's facts. Most of it is the
same on every run and needs no action.

## 2. Decisions

1. **Only what needs attention shows after a generate:** the headline, the warnings and
   errors, and the time table.
2. **The rest is folded under one *Details (n)*,** n the lines in it, closed at first; once
   opened it stays open, as the operations' folded sections do. It holds the notes (the cut
   outline, operation facts, checker notes, the work zero, "one part shown") and, when the
   import had no issues, the drawing's facts.
3. **The whole job's lengths (*In all: …*) leave the window's report:** the Statistics
   sections of the operations and parts have them.
4. **What the estimate leaves out** ("At the programmed feeds …", and the assumed rapid
   rate) is the hover of the total in the time table.
5. **The drawing's facts stand alone, open, when the import has issues;** otherwise they
   are under Details.
6. **Only the window changes:** `chipload run`, `plan` and the setup sheet read as before,
   byte for byte. Reports of other actions (an import, an update) keep their lines in view:
   only a generate's report has a time table and the Details fold.

## 3. As built

- `EstimateTable.hover`: the caveat; `estimate_table` in generation.rs builds the table
  from `estimate_lines(…, Lengths::Without)` and gives the cut outline back as a note.
- `report_body`: with a time table, the Info lines and the drawing's facts without issues
  go into a `CollapsingHeader` (`report-details`, id `report-details`); the
  `OPEN_SECTIONS` flag of the tests opens it.
- Tests: `cycle_time.rs` (the hover, no *In all* line, the Details fold drawn closed).
