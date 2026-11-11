# SPDX-License-Identifier: MPL-2.0
import os
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
