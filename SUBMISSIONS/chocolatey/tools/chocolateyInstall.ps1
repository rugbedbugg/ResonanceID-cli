$ErrorActionPreference = 'Stop'

$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

$packageArgs = @{
    packageName    = 'resonanceid-cli'
    fileFullPath   = Join-Path $toolsDir 'resonanceid-cli.exe'
    url64bit       = 'https://github.com/rugbedbugg/ResonanceID-cli/releases/download/v1.0.1/resonanceid-cli-v1.0.1-windows-x86_64.exe'
    checksum64     = 'B66D5ACA235385526315A8D1A51C89645F09CE1CF90BCE32BB829694F3834F99'
    checksumType64 = 'sha256'
}

# Downloads the binary into the package's tools folder; Chocolatey then
# auto-creates a `resonanceid-cli` shim for the .exe on the PATH.
Get-ChocolateyWebFile @packageArgs
