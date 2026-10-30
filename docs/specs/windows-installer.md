# Windows installer

Status: decisions 1–6 agreed 2026-10-30 (the user: "could we add a windows installer so we
can open chipload files with a double click?", then "go ahead"); 7–10 settled while
writing, as recommended. ADR 0025.

## 1. Why

The Windows download is one `.exe` with nothing to install. It runs, but Windows doesn't
know it opens `.chipload` files: a project can't be opened with a double-click, has no
icon of its own, and Chipload isn't in the Start menu. Windows users expect an installer
that does those things and can be removed again from *Settings > Apps*.

## 2. What it does

`chipload-X.Y.Z-windows-x86_64-setup.exe` (and `…-aarch64-setup.exe` for ARM laptops):

- installs, for the user running it, into `%LOCALAPPDATA%\Programs\Chipload` (they may
  choose another folder): `chipload-gui.exe`, the command line `chipload.exe`, `LICENSE`,
  `THIRD-PARTY-LICENSES.txt` and the uninstaller;
- makes `.chipload` files Chipload projects: Chipload's icon, and a double-click opens
  them in Chipload (`chipload-gui.exe "FILE"`, which the window already takes);
- puts Chipload in the *Open with* list of `.dxf` and `.svg` files, without making it
  their default program;
- adds *Chipload* to the Start menu, and an entry under *Settings > Apps* to remove it;
- finishes with *Run Chipload* ticked.

No administrator rights are asked for. Installing a newer version over an older one
replaces it in place.

The uninstaller removes the files, the shortcut and its own registry entries. It leaves
the user's settings, tool library, machines, materials and posts (in the settings folder,
not the install folder): they belong to the user and outlive a reinstall. A `.chipload`
association is removed only while it still points at Chipload.

## 3. Registry (all under `HKEY_CURRENT_USER`)

| Key | Values |
|---|---|
| `Software\Classes\.chipload` | default `Chipload.Project` |
| `Software\Classes\Chipload.Project` | default "Chipload project" |
| `…\Chipload.Project\DefaultIcon` | default `"INSTDIR\chipload-gui.exe",0` |
| `…\Chipload.Project\shell\open\command` | default `"INSTDIR\chipload-gui.exe" "%1"` |
| `Software\Classes\Applications\chipload-gui.exe\SupportedTypes` | `.chipload`, `.dxf`, `.svg` (empty strings) |
| `Software\Classes\Applications\chipload-gui.exe\shell\open\command` | as above |
| `Software\Classes\.dxf\OpenWithProgids`, `…\.svg\OpenWithProgids` | `Chipload.Drawing` (empty) |
| `Software\Classes\Chipload.Drawing` | default "Drawing", icon and open command as above |
| `Software\Microsoft\Windows\CurrentVersion\Uninstall\Chipload` | `DisplayName`, `DisplayVersion`, `Publisher`, `DisplayIcon`, `InstallLocation`, `UninstallString`, `QuietUninstallString`, `NoModify`, `NoRepair`, `EstimatedSize`, `URLInfoAbout` |

After writing or removing the associations the installer tells Explorer
(`SHChangeNotify(SHCNE_ASSOCCHANGED)`), so icons change without logging out.

## 4. Building

- `packaging/windows/chipload.nsi`, written for this project from the NSIS manual (the
  Modern UI 2 pages, the registry layout above from Microsoft's documentation on file
  types and *Open with*).
- `packaging/windows/build-installer.sh VERSION ARCH GUI_EXE CLI_EXE OUTDIR` runs
  `makensis` on Linux (Ubuntu's `nsis` package): NSIS builds Windows installers on any
  system, so both installers are made in one Linux job from the two Windows builds.
- The installer's own program is NSIS's 32-bit x86 stub for both: it runs on ARM64
  Windows through its emulation, and installs the ARM64 programs.
- Compression **zlib** (decision 8), so everything in the installer is under NSIS's zlib
  licence.
- The release workflow: a job `windows-installer` after the Windows builds makes both
  installers; a job on `windows-latest` installs the x86_64 one silently (`/S /D=…`),
  checks the files and registry values, runs `chipload.exe --version` from the install
  folder, uninstalls silently and checks it's all gone. The draft release waits for both.
  A run by hand builds and tests them too.

## 5. Texts

The installer's pages are in the six languages of the app, chosen from Windows' display
language, English otherwise: NSIS's own translations for its standard pages, and the few
texts of ours (the file type names, *Chipload project*) in the script. The licence page
shows the GPL in English, as `LICENSE` is.

## 6. Not now

- Installing for all users of the computer (needs administrator rights; per-machine
  registry and `Program Files`).
- Adding `chipload.exe` to `PATH` (decision 9).
- Opening a second file in the window already running (decision 5).
- Code signing (decision 6).
- An installer for macOS or Linux: the `.dmg` is already a drag-and-drop install, and an
  AppImage needs none.

## 7. Tests

- `build-installer.sh` in CI for every release and run by hand, with the silent
  install/uninstall test of §4 on Windows.
- Before the first release with it: installed by hand on the user's Windows computer:
  double-click a `.chipload` file, its icon, *Open with* on a DXF, the Start menu entry,
  an upgrade over an installed older one, and removal from *Settings > Apps*.

## 8. Decisions

Agreed 2026-10-30 ("go ahead"):

1. **An installer, and the single-file `.exe` stays** beside it as the portable download.
2. **NSIS** (zlib licence, on the allowed list). Not Inno Setup (its own licence, not on
   the list) nor WiX (MSI: more than needed).
3. **Per user,** no administrator rights.
4. **Only `.chipload` becomes Chipload's;** DXF and SVG get it in *Open with*, their
   default program unchanged.
5. **A second file opens a second window,** as today.
6. **Not code-signed,** like the app: SmartScreen asks once.

Settled while writing (recommended, open to change):

7. **The command line comes with it:** `chipload.exe` beside the app, for scripting and
   AI assistants (the guide's *AI assistants* page gives its path). Whether the separate
   Windows CLI zips then go is the user's to decide.
8. **zlib compression,** not LZMA: NSIS's LZMA module is under the CPL (with an exception
   for installers); zlib keeps the whole installer under one permissive licence. It is a
   few MB larger.
9. **`PATH` unchanged.** NSIS's string length limit can cut a long `PATH` short when it is
   rewritten, which breaks other programs; a guide line with the full path is safe.
10. **The uninstaller keeps the user's settings and libraries.**
