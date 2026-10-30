# 0025: A Windows installer built with NSIS

Status: accepted (2026-10-30). Spec: docs/specs/windows-installer.md (decisions agreed
2026-10-30).

## Context
The Windows download was one portable `.exe`. Windows didn't know it opens `.chipload`
files, so a project couldn't be opened with a double-click, had no icon, and Chipload had
no Start menu entry or uninstaller. The user asked for double-click.

## Decision
- **An installer per architecture, built with NSIS 3** (zlib licence, on the allowed
  list), from a script written for this project (`packaging/windows/chipload.nsi`).
  Inno Setup's licence isn't on the allowed list; WiX (MSI) is more machinery than needed.
- **Built on Linux** (`makensis`, Ubuntu's `nsis`), in one release job from the two Windows
  builds. The installer's own program is NSIS's x86 stub for both architectures; ARM64
  Windows runs it through emulation.
- **zlib compression,** not LZMA: NSIS's LZMA module is under the CPL (with an installer
  exception); with zlib all of NSIS's code in the installer is under its zlib licence.
- **Per user, without administrator rights;** `.chipload` registered as Chipload's, DXF
  and SVG only in *Open with*; the uninstaller keeps the user's settings.
- **The portable `.exe` stays** beside the installer. The Windows command-line zips go:
  the installer carries `chipload.exe`.

## Consequences
- The release workflow gains `windows-installer` (Linux) and `windows-installer-test` (a
  silent install, checks, a silent uninstall, on `windows-latest`); the draft waits for
  both.
- A user moving from the portable `.exe` to the installer keeps their settings: both use
  the same settings folder.
- Not code-signed: SmartScreen warns once, as for the portable `.exe`.
