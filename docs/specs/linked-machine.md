# Spec: a machine from the library, read-only until edited here

Status: approved 2026-10-14 (the user: "I agree with your recommendations").

## 1. Why

User request (2026-10-14): "In the job settings, when choosing a machine from the library,
all options should grey out to show that they come from the library machine?"

Not all of a profile's values belong to the machine. Greying all of them would make the most
common change, the work zero, need an unlink.

## 2. Machine values and job values

| Kind | Values | While linked |
|---|---|---|
| **Machine values** | post, spindle direction, max feed, tool changes, travel | read-only text, with *Edit here* |
| **Job values** | safe Z, work zero, max depth below stock | always editable |

- A profile keeps both. A new project takes all of them; the job values are only where the
  project starts. **Using a profile in an open project** (*Use profile…*, *Use in this
  project*, `chipload machine use`) takes the machine values only, as *Update from library*
  does: changing machine kept resetting the work zero (the user, 2026-10-31; decision 6).
- **The link** (docs/specs/library-in-browser.md §3) compares the machine values only. A
  project whose work zero or safe Z was changed is still *Same*. `chipload machine compare`
  says the same.

## 3. The form

In Job settings > Machine, with a machine linked to a profile that is in the library:

- **Same:** "From machine *Shapeoko*" and the five machine values as plain text, with
  **Edit here**. *Edit here* turns them into the usual fields for this project. The link
  stays. Any change makes it *Differs*.
- **Differs:** the fields, editable. A row that differs from the profile has a small **↺**
  button. Hovering it shows the profile's value, and clicking it takes that value.
  *Update from library* takes them all.
- **Missing, or not linked:** the fields, editable, as today.
- **Safe Z** moves out of the machine rows and becomes a row of its own under Machine
  ("Safe Z (this job)"). The work zero and max depth stay in Stock.

## 4. Decisions

1. **Only the machine values are read-only while linked.** *Approved.*
2. **The job values never count as differing.** *Approved.*
3. **Rows that differ are marked, with a per-row revert.** *Approved.*
4. **Read-only is only how the app shows it:** no format change. `set` and the command line
   can change any value; the link then shows *Differs*. *Approved.*
5. **Edit here** is not saved: reopening the project shows a *Same* machine read-only
   again.
6. **An open project keeps its job values when another profile is used.** *Agreed
   2026-10-31* (the user chose it over keeping only the work zero).

## 5. Tests

- A profile and a project that differ only in safe Z, work zero or max depth are *Same*;
  in spindle, they *Differ* with that one change.
- Applying a profile still reports every value it changed.
- Using a profile in an open project, in the window and with `chipload machine use`, keeps
  its safe Z, work zero and max overcut.

Agreed 2026-10-31 (the user: "the machine properties is a little crammed and hard to
understand"):

7. **One Profile row, one status line.** The *Library* row held four buttons that wrapped
   (and overlapped the next row), and the profile was named three times. Now, for the
   machine and for each project tool alike: **Profile** (a tool: **Library tool**) is a
   dropdown naming the linked entry (or *Use profile…* when there is none); picking another
   uses it. Under it one line: *same as in your library* (no buttons), *differs from your
   library: …* with *Update from library* and *Save to library*, or *not from your
   library* / *… is not in your library* with *Save to library…*. **⋯** beside the
   dropdown holds the rarer *Save as new…* and *Show in library*. Safe Z comes next, then
   the machine values greyed under **From the profile** with *Edit here* beside it. Chosen
   over keeping the buttons, one group per row.
