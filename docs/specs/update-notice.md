# Update notice

Status: draft for review (2026-10-30). The user: "could we also add an update notification
when there are new releases available at GitHub?", then "go ahead with the spec" on the
five recommendations given (decisions 1–5 below). Decisions 6–11 are proposals.

## 1. Why

Chipload changes quickly, and a release often fixes something that matters at the machine.
Today a user only learns of one by looking at the releases page. The window should say,
quietly, when a newer version is out, and where to get it.

## 2. What the user sees

- **Asked once** (decision 1). On a start with no answer stored yet, a bar across the top
  of the canvas asks: *Check for new versions of Chipload? Once a day the app asks GitHub
  for the latest release; nothing about you or your projects is sent.* — **Check** /
  **Don't check**. Nothing is checked before an answer. Closing the bar without choosing
  asks again next start. The answer is a setting under *This computer > Preferences*:
  **Check for new versions**.
- **A newer version** (decision 4): a line in the status bar, right side, *Chipload 0.30.0
  is available*, as a link. Clicking it opens a small menu:
  - **What's new…**: the release's page on GitHub, in the browser;
  - **Skip this version**: no notice for 0.30.0 again, a later one shows.
  The same two are under **Help**, as *Chipload 0.30.0 is available*. Nothing pops up,
  nothing is downloaded or installed (decision 4).
- **Help > Check for new versions now** (decision 8): checks at once, whether the daily
  check is on or off, and says the result in the status bar: *Chipload 0.29.1 is the
  latest version*, the notice above, or *Could not reach GitHub* with the reason in its
  hover.
- **No notice** when offline, when GitHub can't be reached, or when the answer is odd:
  the daily check fails silently and tries again the next day (decision 9).

## 3. The check

- **When** (decision 2): at start, in the background, when on and the last check is more
  than 24 hours ago. The window never waits for it; a slow or hanging request ends after
  10 seconds.
- **What is sent** (decision 3): one HTTPS `GET` to
  `https://api.github.com/repos/chipload/chipload/releases/latest`, with headers
  `User-Agent: chipload/0.29.1` (GitHub's API requires one) and `Accept:
  application/vnd.github+json`. No cookies, no identifier, nothing about the computer,
  the projects or the settings. GitHub sees the IP address, as for any visit to the
  releases page; the bar in §2 says that a request is made.
- **What is read:** `tag_name` (`v0.30.0`) and `html_url` (the release's page). GitHub's
  *latest* is the newest **published** release: never a draft, never a pre-release. The
  page opened is `html_url` only when it starts with
  `https://github.com/chipload/chipload/releases/`, else the releases page itself.
- **Newer** means a higher version, compared as three numbers (`0.30.0` > `0.29.1` >
  `0.29.0`); a tag that isn't `vX.Y.Z` is ignored. Running a version newer than the
  latest release (a build from `main`) shows nothing.
- **Limits:** GitHub allows 60 unauthenticated requests an hour per address; once a day
  per computer is far within it. The response is read up to 256 KB.

## 4. Where it lives

- **The window only** (decision 5, CLAUDE.md: no network calls from the library crates):
  a module `crates/chipload-gui/src/updates.rs` with the request, run on a thread; the
  result comes back to the window as a message. Parsing the answer and comparing versions
  are plain functions there, tested without a network.
- **Not the command line, not the browser build.** `chipload --version` stays offline.
- **Settings** (`settings.json`, optional fields, no version bump):
  - `update_check`: `true`, `false`, or missing (not asked yet);
  - `update_checked_at`: when the last daily check ran, seconds since 1970 (UTC);
  - `update_skipped`: the version skipped, e.g. `"0.30.0"`.
  An older Chipload that saves the settings drops them, and the question comes back once:
  harmless.

## 5. Dependencies (decision 5)

- **`ureq` 3** (MIT OR Apache-2.0), without default features: HTTPS through `rustls`
  (Apache-2.0 OR ISC OR MIT) with `ring` (Apache-2.0 AND ISC), certificates checked
  against the **system's own store** with `rustls-platform-verifier` (MIT OR Apache-2.0).
  Not `webpki-roots`, the bundled certificate list: its licence, CDLA-Permissive-2.0, isn't
  on the allowed list (decision 6). The system's store also follows a company's own
  certificates and updates with the system.
- Only `chipload-gui` depends on it. The exact feature set, every crate it brings and their
  licences are checked with `cargo deny` before it is added, and stated in the commit;
  THIRD-PARTY-LICENSES.txt is regenerated.
- A proxy set in `HTTPS_PROXY` is used (decision 10); how `ureq` 3 reads it is checked in
  its source when this is built.

## 6. Texts

The bar, the status line, the menu entries and the setting, in the six languages
(`gui-updates.ftl`).

## 7. Not now

- Downloading or installing the new version (decision 4); a changelog shown in the app.
- Telling apart releases for one system only (a fix only for Windows still shows on Linux).
- Checks from the command line.

## 8. Tests

- Version comparison: `0.30.0` > `0.29.1`, `0.29.10` > `0.29.9`, equal, older, tags that
  aren't `vX.Y.Z` (`v0.30.0-rc1`, `guide-0.29.1-1`, `latest`), a missing `v`.
- Parsing GitHub's answer: a saved sample response (its shape from GitHub's REST API
  documentation), a missing `tag_name`, a foreign `html_url`, malformed JSON, an oversized
  answer.
- When to check: never asked → no check, the bar; off → no daily check, *Check now*
  still works; on and checked 2 hours ago → none; 25 hours ago → one; a clock set back
  (checked "in the future") → checks.
- Skip: the skipped version shows no notice; a newer one does.
- The window (egui_kittest): the bar's two buttons store the answer; the status line and
  its menu with a fake result; no request is made in tests (the check function is
  replaced by one that returns a set answer).
- One check against the real GitHub by hand before the release that brings it.

## 9. Decisions

Agreed 2026-10-30 ("go ahead with the spec"):

1. **Ask once,** before any check; the answer is a setting in Preferences.
2. **At most once a day,** at start, in the background.
3. **One request** for the latest published release; nothing about the user is sent.
4. **A quiet notice** (status bar and Help) with *What's new…* and *Skip this version*; no
   pop-up, no download, no install.
5. **An HTTPS client in the window only:** `ureq` with `rustls`.

Proposed:

6. **The system's certificate store** (`rustls-platform-verifier`), not `webpki-roots`
   (CDLA-Permissive-2.0, not on the allowed list).
7. **The bar asks on the first start after this version is installed**, also for users
   who had Chipload before: everyone answers once.
8. **Help > Check for new versions now** works with the daily check off: it is the user's
   own action.
9. **A failed daily check is silent;** only *Check now* says why it failed.
10. **`HTTPS_PROXY` is honoured,** for networks that need a proxy.
11. **An ADR** (0026) for the first network access and the HTTPS dependencies.
