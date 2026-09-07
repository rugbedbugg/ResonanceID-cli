$ErrorActionPreference = 'Stop'

$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

$packageArgs = @{
    packageName    = 'resonanceid-cli'
    fileFullPath   = Join-Path $toolsDir 'resonanceid-cli.exe'
    url64bit       = 'https://github.com/rugbedbugg/ResonanceID-cli/releases/download/v1.0.0/resonanceid-cli-v1.0.0-x86_64-pc-windows-msvc.exe'
    checksum64     = '0B3CDE620A8E8FAF3B9BC94A20BC51B4C2DD58F558E774E0D1E292081D924F7D'
    checksumType64 = 'sha256'
}

# Downloads the binary into the package's tools folder; Chocolatey then
# auto-creates a `resonanceid-cli` shim for the .exe on the PATH.
Get-ChocolateyWebFile @packageArgs
