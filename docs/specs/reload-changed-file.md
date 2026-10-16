# Spec: the app reloads a project changed on disk

Status: approved 2026-10-16 as step 2 of docs/specs/mcp.md (§8, decision 6: "ok continue
with step 2"); the details below follow that outline. Built 2026-10-16
(crates/chipload-gui/src/app/disk.rs). As built: a file that loads to what the user has
unsaved in the app counts as saving it (no prompt).

## 1. Why

An AI assistant working through `chipload mcp` (or a script, or a text editor) changes the
project file while it is open in the app. The user wants to watch those changes in the app
without reopening the file.

## 2. Noticing a change

- While a project is open from a file, the app looks at the file's modification time and
  size **once a second**. Polling needs no new dependency and costs one metadata read per
  second.
- The app remembers both when it opens or saves the file. Its own saves therefore never
  count as a change.
- **Settled first:** a change is acted on only once the time and size are the same at two
  looks a second apart. A file being written is never read half-way.
- A file whose content loads to the project the app already has (touched, or rewritten the
  same) changes nothing.
- A file that is gone or unreadable is left alone. The project stays open as it is.
- A prompt already on screen (§4, or another) waits: the file is looked at again after it.

## 3. No unsaved changes: reload quietly

- The file's project replaces the one shown, as **one undo step**: Ctrl+Z goes back to what
  was there before the change. The view (zoom, 3D), the selection (shapes that no longer
  exist drop out) and the open properties stay, and so does the focused operation while it
  still exists.
- The project counts as saved: it is what the file holds.
- The report says: "Reloaded part.chipload: it changed on disk. Ctrl+Z goes back."
- The toolpath goes out of date, as after any edit. With automatic updates on, the new one
  follows by itself.

## 4. Unsaved changes: ask

A dialog in the middle of the window, as for unsaved changes when closing:
- Title: **"part.chipload changed on disk"**.
- Text: "Another program saved the project. Load its version? Your unsaved changes stay one
  Ctrl+Z away."
- **Load it:** as §3. The changes the user hadn't saved are the undo step before it.
- **Keep mine:** nothing changes, and the project stays unsaved. That version of the file
  isn't asked about again, and the next Save overwrites it. A later change to the file
  asks again.
- Closing the dialog is *Keep mine*.

## 5. A file that can't be loaded

When the changed file doesn't load (broken JSON, a newer schema, a failed migration), the
report says so once: "part.chipload changed on disk but can't be loaded: …. The project
shown is unchanged." It isn't repeated until the file changes again.

## 6. Tests

- A file changed on disk is reloaded after it settles. The project isn't dirty, and Ctrl+Z
  brings back the old value.
- The app's own save doesn't reload.
- With unsaved changes, the prompt comes:
  - *Load it* loads, and Ctrl+Z brings back the unsaved edit;
  - *Keep mine* keeps it, and isn't asked again for the same file.
- A broken file gives one error line and changes nothing.
- No project file (a new project) looks at nothing.
