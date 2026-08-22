# Converts an audio file to the format ResonanceID-cli requires:
# mono, 44.1 kHz, 16-bit PCM WAV.
#
# Usage: .\Convert-ToWav.ps1 -InputFile "song.mp3" [-OutputFile "song.wav"]
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InputFile,

    [Parameter(Mandatory = $false)]
    [string]$OutputFile
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error "ffmpeg not found on PATH. Install it first (winget install Gyan.FFmpeg)."
}

if (-not (Test-Path -LiteralPath $InputFile)) {
    Write-Error "Input file not found: $InputFile"
}

if (-not $OutputFile) {
    $OutputFile = [IO.Path]::ChangeExtension($InputFile, ".wav")
}

ffmpeg -y -hide_banner -i $InputFile -ac 1 -ar 44100 -sample_fmt s16 $OutputFile

if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $OutputFile)) {
    Remove-Item -LiteralPath $OutputFile -ErrorAction SilentlyContinue
    Write-Error "ffmpeg failed to convert '$InputFile'. Is the source a valid audio file?"
}

Write-Host "Converted '$InputFile' -> '$OutputFile'"
