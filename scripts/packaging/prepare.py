"""Prepare all package channels from one existing, non-prerelease GitHub release."""
import argparse
import hashlib
import json
import re
import shutil
import urllib.request
from pathlib import Path
from xml.etree import ElementTree as ET

REPO = 'rugbedbugg/ResonanceID-cli'
PACKAGE = 'rugbedbugg.ResonanceID-cli'


def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'ResonanceID-packaging'})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def prepare(tag, output):
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+', tag):
        raise ValueError('Use an existing stable vMAJOR.MINOR.PATCH release tag')
    release = json.loads(fetch(f'https://api.github.com/repos/{REPO}/releases/tags/{tag}'))
    if release['draft'] or release['prerelease'] or release['tag_name'] != tag:
        raise ValueError('Only published stable releases are supported')
    assets = {asset['name']: asset for asset in release['assets']}
    name = f'resonanceid-cli-{tag}-windows-x86_64.exe'
    asset = assets[name]
    url = asset['browser_download_url']
    binary = fetch(url)
    checksum = hashlib.sha256(binary).hexdigest()
    expected = fetch(assets[name + '.sha256']['browser_download_url']).decode().split()[0]
    if checksum.lower() != expected.lower():
        raise ValueError('Windows binary does not match the release checksum')
    version = tag[1:]
    output.mkdir(parents=True, exist_ok=True)
    choco = output / 'chocolatey'
    shutil.copytree('SUBMISSIONS/chocolatey', choco, dirs_exist_ok=True)
    ns = {'n': 'http://schemas.microsoft.com/packaging/2011/08/nuspec.xsd'}
    ET.register_namespace('', ns['n'])
    spec = choco / 'resonanceid-cli.nuspec'
    tree = ET.parse(spec)
    tree.find('n:metadata/n:version', ns).text = version
    tree.find('n:metadata/n:releaseNotes', ns).text = release['html_url']
    tree.write(spec, encoding='utf-8', xml_declaration=True)
    install = choco / 'tools/chocolateyInstall.ps1'
    text = install.read_text()
    text = re.sub(r"url64bit\s*= '[^']+'", f"url64bit = '{url}'", text)
    text = re.sub(r"checksum64\s*= '[^']+'", f"checksum64 = '{checksum}'", text)
    install.write_text(text)
    winget = output / 'winget'
    winget.mkdir(exist_ok=True)
    common = f'PackageIdentifier: {PACKAGE}\nPackageVersion: {version}\n'
    (winget / f'{PACKAGE}.yaml').write_text(common + 'DefaultLocale: en-US\nManifestType: version\nManifestVersion: 1.6.0\n')
    (winget / f'{PACKAGE}.locale.en-US.yaml').write_text(common + '''PackageLocale: en-US
Publisher: Partha Pratim Gogoi
PackageName: ResonanceID CLI
License: MIT
ShortDescription: Offline audio fingerprinting and song recognition CLI.
PackageUrl: https://github.com/rugbedbugg/ResonanceID-cli
LicenseUrl: https://github.com/rugbedbugg/ResonanceID-cli/blob/main/LICENSE
ManifestType: defaultLocale
ManifestVersion: 1.6.0
''')
    (winget / f'{PACKAGE}.installer.yaml').write_text(common + f'''InstallerType: portable
Commands:
- resonanceid-cli
Installers:
- Architecture: x64
  InstallerUrl: {url}
  InstallerSha256: {checksum.upper()}
ManifestType: installer
ManifestVersion: 1.6.0
''')
    aur = output / 'aur'
    aur.mkdir(exist_ok=True)
    source = f'https://github.com/{REPO}/archive/{tag}.tar.gz'
    source_hash = hashlib.sha256(fetch(source)).hexdigest()
    recipe = Path('SUBMISSIONS/aur/PKGBUILD').read_text()
    recipe = re.sub(r'^pkgver=.*$', f'pkgver={version}', recipe, flags=re.M)
    recipe = re.sub(r'^pkgrel=.*$', 'pkgrel=1', recipe, flags=re.M)
    recipe = re.sub(r"^sha256sums=.*$", f"sha256sums=('{source_hash}')", recipe, flags=re.M)
    (aur / 'PKGBUILD').write_text(recipe)
    (output / 'release.json').write_text(json.dumps({'tag': tag, 'version': version, 'repository': REPO, 'windows_url': url, 'windows_sha256': checksum, 'source_sha256': source_hash}, indent=2) + '\n')
    print(f'Prepared {tag}: Chocolatey, Winget and AUR under {output}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--output', type=Path, default=Path('out/packaging'))
    args = parser.parse_args()
    prepare(args.tag, args.output)
