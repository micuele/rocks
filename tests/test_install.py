# SPDX-License-Identifier: MPL-2.0
import os
import re
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class InstallTests(unittest.TestCase):
    def test_installed_launcher_has_no_checkout_dependency(self):
        binary = ROOT/'target/release/rocks'
        if not binary.exists():
            self.skipTest('Build target/release/rocks first')
        with tempfile.TemporaryDirectory(prefix='rocks install ') as directory:
            home = Path(directory)
            data = home/'data with spaces'
            env = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(data))
            subprocess.run([sys.executable, str(ROOT/'install.py'), str(binary)], env=env, check=True, capture_output=True)
            # Run from outside the checkout with only copied resources available to the launcher.
            result = subprocess.run([str(home/'.local/bin/rocks-capture'), '--help'], cwd=home, env=env, check=True, capture_output=True, text=True)
            self.assertIn('--backend', result.stdout)
            desktop = (data/'applications/rocks.searching.Rocks.desktop').read_text()
            self.assertIn('Name=rocks\n', desktop)
            self.assertIn('Icon=rocks.searching.Rocks\n', desktop)
            self.assertNotIn(str(ROOT), desktop)
            self.assertEqual((data/'icons/hicolor/256x256/apps/rocks.searching.Rocks.png').read_bytes(), (ROOT/'assets/rocks.png').read_bytes())
            self.assertEqual((data/'rocks/LICENSE').read_bytes(), (ROOT/'LICENSE').read_bytes())

    def test_system_install_is_staged_for_packages(self):
        binary = ROOT/'target/release/rocks'
        if not binary.exists():
            self.skipTest('Build target/release/rocks first')
        with tempfile.TemporaryDirectory(prefix='rocks stage ') as directory:
            stage = Path(directory)
            # A fake HOME proves packaging never writes into the user's directories.
            env = dict(os.environ, HOME=str(stage/'home'))
            env.pop('XDG_DATA_HOME', None)
            subprocess.run([sys.executable, str(ROOT/'install.py'), '--prefix', '/usr', '--destdir', str(stage/'root'), str(binary)], env=env, check=True, capture_output=True)
            self.assertFalse((stage/'home').exists())
            root = stage/'root/usr'
            installed = sorted(str(path.relative_to(root)) for path in root.rglob('*') if path.is_file())
            self.assertEqual(installed, [
                'bin/rocks', 'bin/rocks-capture', 'bin/rocks-setup-kde',
                'share/applications/rocks.searching.Rocks.desktop',
                'share/icons/hicolor/256x256/apps/rocks.searching.Rocks.png',
                'share/licenses/rocks/LICENSE', 'share/licenses/rocks/NOTICE.md',
                'share/rocks/capture.py', 'share/rocks/config.toml', 'share/rocks/kde_shortcut.py',
            ])
            launcher = (root/'bin/rocks-capture').read_text()
            self.assertIn('ROCKS_EDITOR:-/usr/bin/rocks', launcher)
            self.assertIn('exec /usr/bin/python3 /usr/share/rocks/capture.py', launcher)
            self.assertIn('Exec="/usr/bin/rocks-capture"\n', (root/'share/applications/rocks.searching.Rocks.desktop').read_text())
            self.assertNotIn(str(stage), launcher)
            result = subprocess.run([str(root/'bin/rocks'), '--version'], check=True, capture_output=True, text=True)
            self.assertIn('rocks', result.stdout)


class PackagingTests(unittest.TestCase):
    def test_package_versions_match_cargo(self):
        version = re.search(r'^version = "(.+)"$', (ROOT/'Cargo.toml').read_text(), re.M).group(1)
        self.assertIn(f'\npkgver={version}\n', (ROOT/'packaging/arch/PKGBUILD').read_text())
        self.assertRegex((ROOT/'packaging/rpm/rocks.spec').read_text(), rf'\nVersion:\s+{re.escape(version)}\n')
