# Installer for knightwatch.
# Usage:
#   irm https://github.com/YofaGh/knightwatch/releases/latest/download/install.ps1 | iex
param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.cargo\bin"
)

$Repo = "YofaGh/knightwatch"
$BinName = "knightwatch"
$Target = "x86_64-pc-windows-msvc"
$Archive = "$BinName-$Target.zip"

if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/$Archive"
} else {
    $Url = "https://github.com/$Repo/releases/download/$Version/$Archive"
}

$TmpDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))
$ArchivePath = Join-Path $TmpDir $Archive

Write-Host "Downloading $Url"
Invoke-WebRequest -Uri $Url -OutFile $ArchivePath

Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $TmpDir "$BinName-$Target\$BinName.exe") (Join-Path $InstallDir "$BinName.exe") -Force

Remove-Item -Recurse -Force $TmpDir

Write-Host "Installed $BinName to $InstallDir\$BinName.exe"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your user PATH. Restart your terminal to pick it up."
}
