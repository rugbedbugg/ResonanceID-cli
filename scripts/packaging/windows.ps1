param([Parameter(Mandatory)][ValidateSet('chocolatey','winget')][string]$Channel)
$ErrorActionPreference = 'Stop'
function CheckExit([string]$Step) {
    if ($LASTEXITCODE -ne 0) { throw "$Step failed with exit code $LASTEXITCODE" }
}
if ($Channel -eq 'chocolatey') {
    Push-Location prepared/chocolatey
    choco pack
    CheckExit 'Pack'
    try {
        choco install resonanceid-cli --source . --yes --no-progress
        CheckExit 'Install'
        & "$env:ChocolateyInstall/bin/resonanceid-cli.exe" --help
        CheckExit 'CLI smoke'
    } finally {
        choco uninstall resonanceid-cli --yes --no-progress
        CheckExit 'Uninstall'
        if (Test-Path "$env:ChocolateyInstall/bin/resonanceid-cli.exe") { throw 'Shim survived uninstall' }
        Pop-Location
    }
} else {
    Install-Module Microsoft.WinGet.Client -Repository PSGallery -Force
    Repair-WinGetPackageManager -AllUsers
    winget settings --enable LocalManifestFiles
    CheckExit 'Enable local manifests'
    winget validate --manifest prepared/winget
    CheckExit 'Manifest validation'
    try {
        winget install --manifest prepared/winget --scope user --accept-package-agreements --accept-source-agreements --disable-interactivity
        CheckExit 'Install'
        & "$env:LOCALAPPDATA/Microsoft/WinGet/Links/resonanceid-cli.exe" --help
        CheckExit 'CLI smoke'
    } finally {
        winget uninstall --manifest prepared/winget --scope user --silent --accept-source-agreements --disable-interactivity
        CheckExit 'Uninstall'
        if (Test-Path "$env:LOCALAPPDATA/Microsoft/WinGet/Links/resonanceid-cli.exe") { throw 'Portable alias survived uninstall' }
    }
}
