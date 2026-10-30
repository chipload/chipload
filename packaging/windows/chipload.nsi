; SPDX-License-Identifier: GPL-3.0-or-later
; The Windows installer (docs/specs/windows-installer.md, ADR 0025): Chipload for the user
; running it, .chipload files opened with a double-click, Chipload in Open with for DXF and
; SVG, a Start menu entry and an entry in Settings > Apps to remove it again.
;
; Built by build-installer.sh, which defines VERSION, ARCH, STAGE (the folder holding
; chipload-gui.exe, chipload.exe, LICENSE, THIRD-PARTY-LICENSES.txt and chipload.ico) and
; OUTDIR.

Unicode true
; zlib: everything in the installer stays under NSIS's zlib licence (spec decision 8).
SetCompressor /SOLID zlib
RequestExecutionLevel user

!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !error "build-installer.sh defines VERSION"
!endif
!ifndef ARCH
  !error "build-installer.sh defines ARCH"
!endif
!ifndef STAGE
  !error "build-installer.sh defines STAGE"
!endif
!ifndef OUTDIR
  !error "build-installer.sh defines OUTDIR"
!endif

!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Chipload"
!define CLASSES "Software\Classes"
!define GUI "chipload-gui.exe"
; SHChangeNotify(SHCNE_ASSOCCHANGED): Explorer shows the new icons and Open with at once.
!define ASSOC_CHANGED "System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'"

Name "Chipload"
OutFile "${OUTDIR}\chipload-${VERSION}-windows-${ARCH}-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Chipload"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
BrandingText "Chipload ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=0 "ProductName" "Chipload"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileDescription" "Chipload ${VERSION} setup"
VIAddVersionKey /LANG=0 "LegalCopyright" "Gustav Näslund, GPL-3.0-or-later"

!define MUI_ICON "${STAGE}\chipload.ico"
!define MUI_UNICON "${STAGE}\chipload.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${GUI}"

!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; The app's six languages; NSIS takes the one Windows shows, else the first.
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "French"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "Italian"
!insertmacro MUI_LANGUAGE "Swedish"

; The file type names Explorer shows, as the app names them (crates/chipload-i18n).
LangString PROJECT_TYPE ${LANG_ENGLISH} "Chipload project"
LangString PROJECT_TYPE ${LANG_GERMAN} "Chipload-Projekt"
LangString PROJECT_TYPE ${LANG_FRENCH} "Projet Chipload"
LangString PROJECT_TYPE ${LANG_SPANISH} "Proyecto de Chipload"
LangString PROJECT_TYPE ${LANG_ITALIAN} "Progetto Chipload"
LangString PROJECT_TYPE ${LANG_SWEDISH} "Chipload-projekt"
LangString DRAWING_TYPE ${LANG_ENGLISH} "Drawing"
LangString DRAWING_TYPE ${LANG_GERMAN} "Zeichnung"
LangString DRAWING_TYPE ${LANG_FRENCH} "Dessin"
LangString DRAWING_TYPE ${LANG_SPANISH} "Dibujo"
LangString DRAWING_TYPE ${LANG_ITALIAN} "Disegno"
LangString DRAWING_TYPE ${LANG_SWEDISH} "Ritning"

Section "Chipload"
  SetOutPath "$INSTDIR"
  File "${STAGE}\${GUI}"
  File "${STAGE}\chipload.exe"
  File "${STAGE}\LICENSE"
  File "${STAGE}\THIRD-PARTY-LICENSES.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\Chipload.lnk" "$INSTDIR\${GUI}"

  ; .chipload files are Chipload projects.
  WriteRegStr HKCU "${CLASSES}\.chipload" "" "Chipload.Project"
  WriteRegStr HKCU "${CLASSES}\Chipload.Project" "" "$(PROJECT_TYPE)"
  WriteRegStr HKCU "${CLASSES}\Chipload.Project\DefaultIcon" "" '"$INSTDIR\${GUI}",0'
  WriteRegStr HKCU "${CLASSES}\Chipload.Project\shell\open\command" "" '"$INSTDIR\${GUI}" "%1"'

  ; DXF and SVG: Chipload in Open with, their default program left as it is.
  WriteRegStr HKCU "${CLASSES}\Chipload.Drawing" "" "$(DRAWING_TYPE)"
  WriteRegStr HKCU "${CLASSES}\Chipload.Drawing\DefaultIcon" "" '"$INSTDIR\${GUI}",0'
  WriteRegStr HKCU "${CLASSES}\Chipload.Drawing\shell\open\command" "" '"$INSTDIR\${GUI}" "%1"'
  WriteRegStr HKCU "${CLASSES}\.dxf\OpenWithProgids" "Chipload.Drawing" ""
  WriteRegStr HKCU "${CLASSES}\.svg\OpenWithProgids" "Chipload.Drawing" ""
  WriteRegStr HKCU "${CLASSES}\Applications\${GUI}\shell\open\command" "" '"$INSTDIR\${GUI}" "%1"'
  WriteRegStr HKCU "${CLASSES}\Applications\${GUI}\SupportedTypes" ".chipload" ""
  WriteRegStr HKCU "${CLASSES}\Applications\${GUI}\SupportedTypes" ".dxf" ""
  WriteRegStr HKCU "${CLASSES}\Applications\${GUI}\SupportedTypes" ".svg" ""

  ; Settings > Apps.
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "Chipload"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "Gustav Näslund"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://github.com/chipload/chipload"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" '"$INSTDIR\${GUI}",0'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" "$0"

  ${ASSOC_CHANGED}
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${GUI}"
  Delete "$INSTDIR\chipload.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD-PARTY-LICENSES.txt"
  Delete "$INSTDIR\uninstall.exe"
  ; Only if empty: anything the user put there stays.
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Chipload.lnk"

  ; .chipload only while it is still Chipload's.
  ReadRegStr $0 HKCU "${CLASSES}\.chipload" ""
  StrCmp $0 "Chipload.Project" 0 +2
    DeleteRegKey HKCU "${CLASSES}\.chipload"
  DeleteRegKey HKCU "${CLASSES}\Chipload.Project"
  DeleteRegKey HKCU "${CLASSES}\Chipload.Drawing"
  DeleteRegValue HKCU "${CLASSES}\.dxf\OpenWithProgids" "Chipload.Drawing"
  DeleteRegValue HKCU "${CLASSES}\.svg\OpenWithProgids" "Chipload.Drawing"
  DeleteRegKey HKCU "${CLASSES}\Applications\${GUI}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"

  ; The settings folder (tools, machines, materials, posts) is the user's: it stays.
  ${ASSOC_CHANGED}
SectionEnd
