# Spec: generating in the background, and live preview

Status: approved (2026-10-12; decisions 1–6 as recommended). Items 1 and 2 of the general improvement pass (TODO.md, approved
2026-10-12).

## 1. Why

`generate()` in the GUI calls `run_job` on the UI thread. While a large pocket, rounded
rings or an adaptive pocket plans, the window freezes: no redraw, no Cancel, and the system
may call the app "not responding". And every edit throws the toolpath away; the user has to
press Generate (F5) again to see what the edit did.

## 2. What changes

**Background generation (item 1)**

- Generate copies the project (`Job`) and plans it on a worker thread. The window stays
  live; the toolbar's Generate button turns into **■ Cancel** with a spinner and
  "Generating…" in the status bar.
- One generation at a time. Starting a new one cancels the running one.
- **Cancel** is cooperative: the pipeline checks a flag between operations and inside the
  long loops (pocket pieces, the rounding rounds, adaptive stages, the uncut check) and
  stops with `PipelineError::Cancelled`. A cancelled run leaves nothing behind: no toolpath,
  no report change other than "Generation cancelled".
- The result (toolpath, report, material left, start markers) is applied only if the
  project hasn't changed since the run started. A result for an older project is dropped.
- A panic on the worker thread becomes a report error ("Generation failed unexpectedly:
  …, please report it with the project") and the app keeps running with the project
  intact. (Today such a panic closes the app.)
- The CLI is unchanged: `run_job(job)` stays and calls the cancellable version with a flag
  that is never set. Output is byte-identical; cancelling only stops, it never changes
  what a finished run produces.

**Live preview (item 2)**

- After an edit, the toolpath regenerates by itself once the project has been unchanged for
  **0.5 s** (each new edit restarts the wait, so typing a number or dragging a handle
  generates once at the end, not at every step).
- Until the new result arrives, the old toolpath stays visible, **faded**, with the
  "Toolpath out of date" badge (today it's removed). Material left and start markers of
  the old result are hidden, since they would be wrong.
- **Export G-code** is enabled only when the shown toolpath is current and passed the
  check. While generating, Ctrl+E waits for the run and then exports (or reports why it
  can't).
- Automatic runs don't flash errors: an incomplete project (e.g. an operation with no
  shapes yet) shows its problems in the report as now, but without the red status-bar
  message. Generate (F5) still reports loudly.
- **View → Update toolpath automatically**, on by default, remembered in the app settings.
  Off: today's behaviour (press Generate), but still in the background.

## 3. Files

- **App settings schema 3:** `auto_generate: bool`. The settings struct refuses unknown
  fields, so an older Chipload would fail on the new field: a bump, with `v2_to_v3` setting
  it to true. No project, library or G-code change.

## 4. Decisions to review

1. **Worker thread, one run at a time, newest wins** (a new run cancels the old).
   *Recommended:* simple, and the newest project is the only one worth showing.
2. **Cooperative cancel** checked between operations and in the long loops, through the
   pipeline (`run_job_cancellable(job, &Cancel)`), not killing the thread. *Recommended:*
   Rust can't safely kill a thread; the checks are cheap.
3. **Live preview on by default, 0.5 s after the last edit**, with a View menu switch.
   *Recommended:* the toolpath answers each change without a click; 0.5 s is long enough
   not to run on every keystroke.
4. **Keep the old toolpath faded while out of date** (material left and start markers
   hidden). *Recommended:* the picture doesn't jump to empty on every edit.
5. **Quiet errors on automatic runs**, loud on F5. *Recommended:* an operation that is
   half set up shouldn't flash red while the user is still filling it in.
6. **Settings schema 3** for the switch. *Recommended* (CLAUDE.md: the settings struct
   refuses unknown fields, so adding one needs a bump).

## 5. Tests

- Pipeline: a job run with the flag already set returns `Cancelled`; a run with a flag
  never set gives byte-identical G-code to `run_job` (the golden test keeps covering it).
- Cancel during a run: a large pocket job is cancelled from another thread and returns
  `Cancelled` quickly (well under the full planning time).
- GUI: generating applies the result once it arrives (a test helper waits for the worker);
  a result for a project that changed meanwhile is dropped; an edit with auto-generate on
  starts a run only after the quiet time; Export is disabled while out of date.
- GUI: a panic in the worker becomes a report error and the app keeps its project (a test
  hook that panics inside the worker).
- Settings: a frozen v2 settings file migrates to `auto_generate: true`.

## 6. As built (2026-10-12)

- `chipload_toolpath::Cancel` (an atomic flag) and `ToolpathError::Cancelled`;
  `plan_job_cancellable` and `chipload_pipeline::run_job_cancellable` (`run_job` and
  `plan_job` call them with a flag that is never set). Checked before each operation and
  copy, each pocket piece, each attempt at a piece (also between offset retries), each
  rounding round and the uncut check, and before posting.
- **Not yet inside adaptive planning:** a long adaptive piece finishes its piece before it
  stops. Adaptive is being replaced (branch `adaptive-stage2-wip`); its checks go in with
  the new code.
- GUI `app/generation.rs`: the worker thread (`catch_unwind` around the run), newest wins,
  results applied only to the project they were made from, quiet and loud errors, Ctrl+E
  while generating exports when done. The toolbar's Generate turns into ■ Cancel; the
  status bar shows a spinner; the canvas badge says "Updating the toolpath…" while the
  live preview is on its way.
- The live preview starts after an edit, undo or redo, not when a project is opened or a
  DXF imported (the import report stays on screen).
- App settings schema 3 (`auto_generate`, `settings_v2_to_v3`, frozen sample
  `tests/corpus/settings/v2`).
