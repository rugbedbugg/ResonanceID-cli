# End-to-end demo: convert, store, and recognize using ResonanceID-cli.
#
# Usage:
#   .\Invoke-ResonanceDemo.ps1 -Reference "song.mp3" -Clip "clip.wav"
#   .\Invoke-ResonanceDemo.ps1 -Reference "song.mp3" -Clip "clip.wav" -Binary ..\target\release\resonanceid-cli.exe
#
# The reference track is converted (if needed), indexed into the DB, then the
# clip is recognized against it.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Reference,

    [Parameter(Mandatory = $true)]
    [string]$Clip,

    [Parameter(Mandatory = $false)]
    [string]$Binary = ".\target\release\resonanceid-cli.exe",

    [Parameter(Mandatory = $false)]
    [string]$Title = "",

    [Parameter(Mandatory = $false)]
    [string]$Artist = ""
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

if (-not (Test-Path -LiteralPath $Binary)) {
    Write-Host "Binary not found at '$Binary', building..."
    cargo build --release
}

$referenceWav = $Reference
if ([IO.Path]::GetExtension($Reference) -ne ".wav") {
    & (Join-Path $scriptDir "Convert-ToWav.ps1") -InputFile $Reference
    $referenceWav = [IO.Path]::ChangeExtension($Reference, ".wav")
}

if (-not $Title) { $Title = [IO.Path]::GetFileNameWithoutExtension($Reference) }
if (-not $Artist) { $Artist = "Unknown Artist" }

Write-Host "`n=== Storing reference: $Title by $Artist ==="
& $Binary store $referenceWav $Title $Artist

Write-Host "`n=== Recognizing clip: $Clip ==="
& $Binary recognize $Clip
