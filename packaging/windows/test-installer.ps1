# SPDX-License-Identifier: GPL-3.0-or-later
# Install the Windows installer silently, check what it did, uninstall it and check that it
# is gone but the user's settings stay (docs/specs/windows-installer.md §4, §7). Run on a
# Windows machine without Chipload installed: the release workflow's windows-installer-test.
#
# Usage: packaging/windows/test-installer.ps1 <setup.exe> <version>
param([Parameter(Mandatory)] [string] $Setup, [Parameter(Mandatory)] [string] $Version)
$ErrorActionPreference = "Stop"

function Check([bool] $ok, [string] $what) {
    if (-not $ok) { throw "installer test failed: $what" }
    Write-Output "ok: $what"
}
function Default([string] $key) {
    $item = Get-Item -Path "HKCU:\Software\Classes\$key" -ErrorAction SilentlyContinue
    if ($null -eq $item) { return $null }
    $item.GetValue("")
}

# A folder with a space: every path the installer writes must be quoted.
$dir = Join-Path $env:RUNNER_TEMP "Chipload test"
$gui = Join-Path $dir "chipload-gui.exe"
$open = "`"$gui`" `"%1`""
$shortcut = Join-Path ([Environment]::GetFolderPath("Programs")) "Chipload.lnk"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Chipload"

# /D must come last, unquoted, even with spaces (NSIS).
Start-Process -Wait -FilePath $Setup -ArgumentList "/S", "/D=$dir"

foreach ($file in "chipload-gui.exe", "chipload.exe", "LICENSE", "THIRD-PARTY-LICENSES.txt", "uninstall.exe") {
    Check (Test-Path (Join-Path $dir $file)) "installed $file"
}
Check (Test-Path $shortcut) "Start menu shortcut"
Check ((Default ".chipload") -eq "Chipload.Project") ".chipload is a Chipload project"
Check ((Default "Chipload.Project\shell\open\command") -eq $open) "a double-click opens it: $open"
Check ((Default "Chipload.Project\DefaultIcon") -eq "`"$gui`",0") "its icon"
Check ($null -ne (Get-ItemProperty "HKCU:\Software\Classes\.dxf\OpenWithProgids" -Name "Chipload.Drawing" -ErrorAction SilentlyContinue)) "Open with for DXF"
Check ($null -ne (Get-ItemProperty "HKCU:\Software\Classes\.svg\OpenWithProgids" -Name "Chipload.Drawing" -ErrorAction SilentlyContinue)) "Open with for SVG"
$apps = Get-ItemProperty $uninstallKey
Check ($apps.DisplayVersion -eq $Version) "Settings > Apps lists Chipload $Version"
Check ($apps.InstallLocation -eq $dir) "its install folder"

$cli = & (Join-Path $dir "chipload.exe") --version
Check ($cli -match [regex]::Escape($Version)) "the command line runs: $cli"

# The settings folder, as the command line names it: the uninstaller must leave it.
$library = (& (Join-Path $dir "chipload.exe") tool list | Select-Object -First 1) -replace "^Tool library: ", ""
$settings = Split-Path $library
New-Item -ItemType Directory -Force $settings | Out-Null
$kept = Join-Path $settings "installer-test.txt"
Set-Content $kept "the user's"

# _? runs the uninstaller in place and waits for it (otherwise it copies itself and returns).
Start-Process -Wait -FilePath (Join-Path $dir "uninstall.exe") -ArgumentList "/S", "_?=$dir"

foreach ($file in "chipload-gui.exe", "chipload.exe", "LICENSE", "THIRD-PARTY-LICENSES.txt") {
    Check (-not (Test-Path (Join-Path $dir $file))) "removed $file"
}
Check (-not (Test-Path $shortcut)) "removed the shortcut"
Check ($null -eq (Default ".chipload")) "removed .chipload"
Check (-not (Test-Path "HKCU:\Software\Classes\Chipload.Project")) "removed Chipload.Project"
Check (-not (Test-Path "HKCU:\Software\Classes\Chipload.Drawing")) "removed Chipload.Drawing"
Check ($null -eq (Get-ItemProperty "HKCU:\Software\Classes\.dxf\OpenWithProgids" -Name "Chipload.Drawing" -ErrorAction SilentlyContinue)) "removed Open with for DXF"
Check (-not (Test-Path $uninstallKey)) "removed from Settings > Apps"
Check (Test-Path $kept) "kept the user's settings folder"

Remove-Item $kept
Remove-Item -Recurse -Force $dir
Write-Output "installer test passed"
