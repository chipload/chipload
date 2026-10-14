# 0018: Project tools and machines link to the library by uid

Status: accepted (2026-10-14). Spec: docs/specs/library-in-browser.md. Amends ADR 0014.

## Context
ADR 0014 copies library tools and machine profiles into projects and records the entry's
`name`, to offer "update from library". A rename in the library broke that link, and with the
library now in the Browser the link between a copy and its entry is shown all the time.

## Decision
Every library entry gets a `uid` (16 hex digits), which never changes. Project tools and the
project machine record it as `library_uid`. Projects go to schema 23, `tools.json` and
`machines.json` to version 3.

A uid is FNV-1a 64 of the entry's lower-cased name when it is made, with "#2", "#3" … added
while it is taken. No clock and no random numbers: migrations stay pure, and the project
migration gives a named tool the same uid that the library migration gives the entry of that
name, so existing projects stay linked. The function is frozen.

## Consequences
- Renaming a library entry keeps projects linked. The project's `name` stays as the name
  the entry had when copied.
- Two computers that make an entry of the same name give it the same uid: a project moved
  between them links to that entry, and the Browser says when its values differ.
- Import keeps the file's uids, gives a new one to an entry whose uid is taken, and keeps
  the library's uid when it replaces an entry.
- The link is a note: planning and G-code never read it, as before.
