# Install easyscript, the app and its command line, from the latest release on GitHub. For
# you alone: no administrator, and nothing to install first. In PowerShell:
#
#   irm https://raw.githubusercontent.com/murparreira/easyscript/main/scripts/install.ps1 | iex
#
# Run it again to update. The app goes in the Start menu; the command line `easyscript` to
# %LOCALAPPDATA%\easyscript-cli, added to your PATH. $env:EASYSCRIPT_VERSION = "v0.1.0"
# picks a release instead of the latest.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
# GitHub speaks TLS 1.2; Windows PowerShell 5 may not offer it by default.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$repo = if ($env:EASYSCRIPT_REPO) { $env:EASYSCRIPT_REPO } else { "murparreira/easyscript" }
$api = if ($env:EASYSCRIPT_API) { $env:EASYSCRIPT_API } else { "https://api.github.com" }

function Say($text) { Write-Host "`n$text" -ForegroundColor White }
function Fail($text) { Write-Host "`nerror: $text" -ForegroundColor Red; throw $text }

if (-not [Environment]::Is64BitOperatingSystem) { Fail "easyscript needs 64-bit Windows" }

Say "Finding the release"
$query = if ($env:EASYSCRIPT_VERSION) { "$api/repos/$repo/releases/tags/$($env:EASYSCRIPT_VERSION)" } else { "$api/repos/$repo/releases?per_page=1" }
try {
  $release = @(Invoke-RestMethod -Uri $query -Headers @{ Accept = "application/vnd.github+json" })[0]
} catch {
  Fail "cannot reach GitHub to find the release ($query)"
}
if (-not $release) { Fail "no release of $repo has files to install yet" }
function Pick($pattern, $what) {
  $asset = $release.assets | Where-Object { $_.name -match $pattern } | Select-Object -First 1
  if (-not $asset) { Fail "the release $($release.tag_name) has no $what" }
  $asset.browser_download_url
}
$appUrl = Pick "_x64-setup\.exe$" "installer for Windows"
$cliUrl = Pick "easyscript-cli-windows-x64\.zip$" "command line for Windows"

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("easyscript-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Say "Downloading easyscript $($release.tag_name)"
  Invoke-WebRequest -Uri $appUrl -OutFile (Join-Path $tmp "setup.exe")
  Invoke-WebRequest -Uri $cliUrl -OutFile (Join-Path $tmp "cli.zip")

  Say "Installing"
  # The installer is per user; /S installs without asking.
  $process = Start-Process -FilePath (Join-Path $tmp "setup.exe") -ArgumentList "/S" -Wait -PassThru
  if ($process.ExitCode -ne 0) { Fail "the installer stopped with code $($process.ExitCode)" }

  Expand-Archive -Path (Join-Path $tmp "cli.zip") -DestinationPath $tmp -Force
  $bin = Join-Path $env:LOCALAPPDATA "easyscript-cli"
  New-Item -ItemType Directory -Force -Path $bin | Out-Null
  $exe = Get-ChildItem $tmp -Recurse -Filter "easyscript.exe" | Select-Object -First 1
  Copy-Item $exe.FullName $bin -Force
  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if (-not $userPath) { $userPath = "" }
  if (($userPath -split ";") -notcontains $bin) {
    [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";"), $bin) -join ";").TrimStart(";"), "User")
    Write-Host "Added $bin to your PATH (new terminals see it)."
  }
  Write-Host "The app:          easyscript in the Start menu"
  Write-Host "The command line: easyscript"
  Say "Done: easyscript $($release.tag_name). Run this again to update."
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
