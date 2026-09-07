import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from prepare import prepare


class PreparationTests(unittest.TestCase):
    def test_invalid_tag_never_downloads(self):
        with patch('prepare.fetch') as fetch:
            with self.assertRaises(ValueError):
                prepare('v1.0.1;echo bad', Path('unused'))
            fetch.assert_not_called()

    def test_bad_release_checksum_is_rejected(self):
        release = {'draft': False, 'prerelease': False, 'tag_name': 'v1.0.1', 'assets': [
            {'name': 'resonanceid-cli-v1.0.1-windows-x86_64.exe', 'browser_download_url': 'binary'},
            {'name': 'resonanceid-cli-v1.0.1-windows-x86_64.exe.sha256', 'browser_download_url': 'hash'}]}
        with patch('prepare.fetch', side_effect=[json.dumps(release).encode(), b'binary', b'bad']):
            with tempfile.TemporaryDirectory() as temp:
                with self.assertRaises(ValueError):
                    prepare('v1.0.1', Path(temp))
                self.assertEqual(list(Path(temp).iterdir()), [])

    def test_all_channels_use_the_selected_release(self):
        checksum = hashlib.sha256(b'binary').hexdigest()
        url = 'https://github.com/rugbedbugg/ResonanceID-cli/releases/download/v2.3.4/app.exe'
        release = {'draft': False, 'prerelease': False, 'tag_name': 'v2.3.4', 'html_url': 'https://example.com/release', 'assets': [
            {'name': 'resonanceid-cli-v2.3.4-windows-x86_64.exe', 'browser_download_url': url},
            {'name': 'resonanceid-cli-v2.3.4-windows-x86_64.exe.sha256', 'browser_download_url': 'hash'}]}
        with patch('prepare.fetch', side_effect=[json.dumps(release).encode(), b'binary', checksum.encode(), b'source']):
            with tempfile.TemporaryDirectory() as temp:
                output = Path(temp)
                prepare('v2.3.4', output)
                self.assertIn('<version>2.3.4</version>', (output / 'chocolatey/resonanceid-cli.nuspec').read_text())
                self.assertIn(checksum, (output / 'chocolatey/tools/chocolateyInstall.ps1').read_text())
                self.assertIn(checksum.upper(), (output / 'winget/rugbedbugg.ResonanceID-cli.installer.yaml').read_text())
                self.assertIn('pkgver=2.3.4', (output / 'aur/PKGBUILD').read_text())
                self.assertIn(hashlib.sha256(b'source').hexdigest(), (output / 'aur/PKGBUILD').read_text())


if __name__ == '__main__':
    unittest.main()
