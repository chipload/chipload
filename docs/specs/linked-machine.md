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

- A profile keeps both. Applying it sets all of them, as now; the job values are only where
  the project starts.
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

## 5. Tests

- A profile and a project that differ only in safe Z, work zero or max depth are *Same*;
  in spindle, they *Differ* with that one change.
- Applying a profile still reports every value it changed.
