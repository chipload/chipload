# Spec: the preview is of the active part

Status: approved 2026-10-14 (the user: "go ahead").

## 1. Why

User request (2026-10-14): "When part 2 is active we don't want to simulate or regenerate
part 1. We want only to simulate part 2 then."

## 2. Behaviour

1. **Preview:** with two or more parts that are cut, Generate, the automatic update, the
   simulation and playback plan only the **active part**: the project is planned as if the
   other parts were switched off. Their shapes stay on the canvas; their toolpath doesn't
   show. The stock is simulated from uncut, as a part's cuts are in its own place.
   With one part, or an active part that is switched off, the whole project is planned, as
   before.
2. **Export always writes every part that is cut** (one part alone: its right-click menu,
   docs/specs/part-export.md). When the toolpath shown is of one part,
   Export plans the whole project first (in the background, as Generate does), then writes
   it; the status says "Planning every part, then exporting". The setup sheet is of the
   whole project too.
3. **All parts** on the canvas bar (shown with two or more parts that are cut) previews
   every part. It is off at start and not saved. Switching it plans again at once.
4. **The status bar** says what the toolpath is of: "part 2 of 2 · cycle 4 min 10 s" or
   "all parts · cycle 9 min". The report says "Shown: part "B" only" when it is.
5. **No format change.** The command line plans the whole project as before.

## 3. Tests

- Two parts, the second active: Generate plans only its operations; All parts plans
  every operation; Export from the part's preview writes what `run_job` gives for the
  whole project.
