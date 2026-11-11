# SPDX-License-Identifier: MPL-2.0
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
import capture


class CaptureTests(unittest.TestCase):
    def available(self, *commands):
        return lambda name: name if name in commands else None

    def test_auto_respects_desktop_and_display_protocol(self):
        all_commands = self.available(*capture.BACKENDS)
        self.assertEqual(capture.choose_backend("auto", {"XDG_CURRENT_DESKTOP": "KDE", "WAYLAND_DISPLAY": "wayland-0"}, all_commands), "spectacle")
        self.assertEqual(capture.choose_backend("auto", {"WAYLAND_DISPLAY": "wayland-0"}, all_commands), "grim")
        self.assertEqual(capture.choose_backend("auto", {"DISPLAY": ":0"}, all_commands), "gnome-screenshot")
        self.assertEqual(capture.choose_backend("auto", {"DISPLAY": ":0"}, self.available("import")), "import")
        # Do not silently select an X11 capture command on Wayland.
        with self.assertRaises(RuntimeError):
            capture.choose_backend("auto", {"WAYLAND_DISPLAY": "wayland-0"}, self.available("import"))

    def test_explicit_backend_and_safe_filename_arguments(self):
        self.assertEqual(capture.choose_backend("grim", {}, self.available("grim")), "grim")
        with self.assertRaises(RuntimeError):
            capture.choose_backend("grim", {}, self.available())
        filename = Path('/tmp/a space;$(no-shell).png')
        for backend in capture.BACKENDS:
            command = capture.capture_command(backend, filename)
            self.assertEqual(command[0], backend)
            self.assertEqual(command[-1], str(filename))

    def test_capture_must_be_a_complete_png(self):
        with tempfile.TemporaryDirectory() as directory:
            image = Path(directory)/"image.png"
            image.write_bytes(b'\x89PNG\r\n\x1a\npartial')
            with self.assertRaises(RuntimeError):
                capture.wait_png(image, seconds=0)
            image.write_bytes(b'\x89PNG\r\n\x1a\n\0\0\0\0IEND\xaeB`\x82')
            capture.wait_png(image, seconds=0)

    def test_unavailable_monitor_geometry_preserves_entire_capture(self):
        with patch('capture.subprocess.check_output', side_effect=FileNotFoundError):
            self.assertEqual(capture.kde_monitors(Path('/unused.png')), '')

    def test_editor_comes_from_launcher_then_path(self):
        self.assertEqual(capture.find_editor({'ROCKS_EDITOR': '/opt/rocks'}, self.available('rocks')), '/opt/rocks')
        self.assertEqual(capture.find_editor({}, lambda name: '/usr/bin/rocks' if name == 'rocks' else None), '/usr/bin/rocks')
        with self.assertRaises(RuntimeError):
            capture.find_editor({}, self.available())


if __name__ == '__main__':
    unittest.main()
