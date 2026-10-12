# Spec: autosave, crash recovery and a backup on save

Status: draft for review. Items 3 and 4 of the general improvement pass (TODO.md, approved
2026-10-12).

## 1. Why

If Chipload crashes, the machine loses power or the window is killed, every change since
the last save is gone. And saving replaces the project file with no way back to the
version before, unlike the tool library and machine profiles, which keep a `.bak`.

## 2. Recovery copies

**What is written**

- While the project has unsaved changes, a **recovery copy** is written **2 s after each
  finished edit** (an edit as recorded for undo: after a drag or a typed value, not during
  it). Two seconds gathers a burst of edits into one write.
- The copy is an ordinary project file (`save_job`, current schema), so it opens anywhere,
  the CLI included. Beside it, a small **recovery note** says where it belongs: the
  project's path (none for a project never saved) and the Chipload version. The note is a
  new persisted format, versioned from the start (`schema_version` 1, `written_by`).
- An edit that leaves the project incomplete (one that couldn't be saved, e.g. a feed of 0
  while you type) isn't written; the copy keeps the last complete state.
- Everything goes in the settings folder, `recovery/`, one set per running window:
  `<id>.chipload`, `<id>.json` (the note) and `<id>.lock`. `<id>` is unique per session.
  Nothing is written next to the user's projects.

**When it is removed**

- On save, on closing (after Save or Don't save), and when another project is opened or a
  new one started (the old one was saved or deliberately discarded): that session's
  recovery copy is deleted.
- A project with no unsaved changes has no recovery copy.

**After a crash**

- Each window holds an OS lock on its `<id>.lock` for as long as it runs (`File::try_lock`,
  in the standard library); the OS releases it when the process ends, crash or not. At
  start, every recovery set whose lock can be taken belongs to no running window: it is
  left over.
- Chipload then shows **Recover unsaved work**: for each left-over copy, the project's
  name and path (or "never saved"), when it was written, and whether the project file
  has changed since. Buttons: **Open** (opens the copy as an unsaved project with the
  original path, so Ctrl+S saves over the original), **Delete**, **Later** (keep, ask at
  the next start).
- A copy that can't be read (cut short, from a newer Chipload) is listed with the reason,
  and only offers Delete or Later; never deleted without asking.

**Panics:** the recovery copy is at most 2 s older than the last finished edit, so a panic
on the UI thread loses at most that. Planning runs on the worker thread, where a panic is
already caught (docs/specs/background-generation.md). No panic hook: it would need the
project in shared global state, which CLAUDE.md rules out, for little gain.

## 3. A backup on save

- Saving a project over an existing file first keeps that file as **`<name>.chipload.bak`**
  (one generation, replaced each save), then writes the new one as now (temporary file and
  rename). In `save_job`, so the CLI commands that change a project in place (`place`,
  `copies`, `op tool`, `update`) keep one too.
- The first save of a new file makes no `.bak`. A failure to write the `.bak` stops the
  save with an error, and the project file is untouched.
- File > **Revert to backup…** is not part of this; the `.bak` is opened like any project
  (rename it, or File > Open path…). It can be added later.

## 4. CLI

Recovery belongs to the GUI's editing session; the CLI never holds unsaved changes, so it
has nothing to recover. The copies are ordinary project files the CLI can `inspect` and
`run`. The `.bak` applies to the CLI as above.

## 5. Decisions to review

1. **Write 2 s after each finished edit**, not on a fixed 30 s timer. *Recommended:* loses at
   most the last 2 s, writes only when something changed.
2. **No panic hook** (changed from the improvement list): the 2 s copy covers a UI-thread
   panic, and a hook would need global state. *Recommended.*
3. **Recovery copies in the settings folder**, with a note giving the project's path, not
   next to the project. *Recommended:* never litters project folders, works for projects
   never saved and for read-only folders.
4. **The last complete state** when an edit leaves the project unsaveable. *Recommended:*
   the copy is always a valid project file.
5. **OS file locks** to tell a crashed session from a second window that is still open.
   *Recommended:* standard library, no new dependency; the OS releases locks on a crash.
6. **Open restores as unsaved, with the original path**; Delete and Later as above.
   *Recommended.*
7. **`.bak` in `save_job`**, one generation, for the GUI and the CLI. *Recommended:* the
   same rule as the libraries.

## 6. Tests

- Job crate: saving over a file keeps the old one as `.bak` (byte-identical); a first save
  makes none; an unwritable `.bak` stops the save and leaves the project intact. The
  recovery note round-trips; a newer or malformed note is refused with a typed error.
- GUI: an edit writes a copy after the quiet time and not before; an incomplete edit keeps
  the last complete copy; saving, opening another project and closing delete it.
- Left-over detection: a set whose lock is held (as by a running window) is not offered; a
  set with a free lock is; Open restores it as unsaved with the original path; Delete
  removes all three files; a corrupt copy is listed and not deleted.
