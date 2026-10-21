# Build easyscript from this source tree and install it for you, in one command, in
# PowerShell:
#
#   powershell -ExecutionPolicy Bypass -File scripts\build-from-source.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\build-from-source.ps1 -NoInstall   (build only)
#
# The app is installed for you alone (no administrator), in the Start menu. The command line
# `easyscript.exe` goes to %LOCALAPPDATA%\easyscript-cli, added to your PATH.
#
# Needs Rust (https://rustup.rs, with the Visual Studio C++ build tools it asks for) and
# Node 22 (https://nodejs.org); with mise (https://mise.jdx.dev) installed, the script gets
# both itself. WebView2 comes with Windows 10 and 11.
param([switch]$NoInstall)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Say($text) { Write-Host "`n$text" -ForegroundColor White }
function Fail($text) { Write-Host "`nerror: $text" -ForegroundColor Red; exit 1 }
function Run($command, [string[]]$arguments) {
  & $command @arguments
  if ($LASTEXITCODE -ne 0) { Fail "$command $($arguments -join ' ') failed" }
}

# Rust and Node: from mise when it is there (mise.toml pins them), else from the PATH.
if (Get-Command mise -ErrorAction SilentlyContinue) {
  Say "Rust and Node, with mise"
  mise trust --quiet (Join-Path $PWD "mise.toml") 2>$null
  Run mise @("install")
  mise env --shell pwsh | Out-String | Invoke-Expression
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Fail "Rust is missing. Install it from https://rustup.rs, or install mise (https://mise.jdx.dev) and run this again." }
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) { Fail "Node is missing. Install Node 22 from https://nodejs.org, or install mise (https://mise.jdx.dev) and run this again." }
$nodeMajor = [int](node -p "process.versions.node.split('.')[0]")
if ($nodeMajor -lt 20) { Fail "Node $nodeMajor is too old: Node 22 is needed." }

Say "JavaScript packages"
Run npm @("ci", "--no-audit", "--no-fund")

Say "The command line (a few minutes the first time)"
Run npm @("run", "build:page")
Run cargo @("build", "--release", "--locked", "-p", "es-cli")

Say "The app (a few minutes the first time)"
Run npm @("run", "tauri", "--", "build", "--bundles", "nsis")

if ($NoInstall) {
  Say "Built"
  Write-Host "The app:          target\release\easyscript-app.exe [a game folder]"
  Write-Host "The installer:    target\release\bundle\nsis"
  Write-Host "The command line: target\release\easyscript.exe"
  exit 0
}

Say "Installing"
$setup = Get-ChildItem "target\release\bundle\nsis\*-setup.exe" | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $setup) { Fail "the installer was not built" }
# The installer is per user; /S installs without asking.
$process = Start-Process -FilePath $setup.FullName -ArgumentList "/S" -Wait -PassThru
if ($process.ExitCode -ne 0) { Fail "the installer stopped with code $($process.ExitCode)" }

$bin = Join-Path $env:LOCALAPPDATA "easyscript-cli"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item "target\release\easyscript.exe" $bin -Force
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
if (($userPath -split ";") -notcontains $bin) {
  [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";"), $bin) -join ";").TrimStart(";"), "User")
  Write-Host "Added $bin to your PATH (new terminals see it)."
}
Write-Host "The app:          easyscript in the Start menu"
Write-Host "The command line: easyscript"
Say "Done. Run this again to update after pulling new changes."
