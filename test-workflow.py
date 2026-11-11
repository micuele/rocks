#!/usr/bin/env python3
"""Exercise the fork on an isolated X11 display (xvfb-run + xdotool + xclip)."""
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import time
from urllib.parse import unquote, urlparse
import zlib


def run(*args):
    return subprocess.check_output(args, timeout=10)


def window(pattern):
    for _ in range(100):
        result = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", pattern], capture_output=True)
        if result.returncode == 0:
            return result.stdout.decode().splitlines()[-1]
        time.sleep(0.1)
    raise AssertionError(f"Window did not appear: {pattern}")


def key(*keys):
    run("xdotool", "key", "--clearmodifiers", *keys)
    time.sleep(0.2)


def drag(editor, x, y, button=1, dx=170, dy=95):
    run("xdotool", "windowfocus", "--sync", editor)
    run("xdotool", "mousemove", "--window", editor, str(x), str(y), "mousedown", str(button))
    run("xdotool", "mousemove", "--sync", "--window", editor, str(x + dx), str(y + dy))
    time.sleep(0.2)
    run("xdotool", "mouseup", str(button))


def region(editor, number, text):
    drag(editor, 370 + number * 25, 130 + number * 60)
    dialog = window(f"Region {number}")
    run("xdotool", "windowfocus", "--sync", dialog)
    run("xdotool", "type", "--clearmodifiers", text)
    key("Return")


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def main():
    binary = str(Path(sys.argv[1]).resolve())
    with tempfile.TemporaryDirectory(prefix="rocks-test-") as directory:
        directory = Path(directory)
        png = b"\x89PNG\r\n\x1a\n"
        png += chunk(b"IHDR", struct.pack(">IIBBBBB", 800, 600, 8, 2, 0, 0, 0))
        png += chunk(b"IDAT", zlib.compress((b"\0" + b"\xf5\xf5\xf5" * 800) * 600))
        png += chunk(b"IEND", b"")
        image = directory / "input.png"
        image.write_bytes(png)
        env = dict(os.environ, GDK_BACKEND="x11", LIBGL_ALWAYS_SOFTWARE="1")
        env.pop("WAYLAND_DISPLAY", None)
        log = (directory / "editor.log").open("w+")
        process = subprocess.Popen([binary, "--filename", str(image), "--initial-tool", "rectangle",
            "--actions-on-enter", "save-to-clipboard", "--actions-on-escape", "exit",
            "--early-exit", "copy", "--disable-notifications", "--title", "rocks Workflow Test"],
            env=env, stdout=log, stderr=log)
        try:
            editor = window("^rocks Workflow Test$")
            region(editor, 1, "Fix the header")
            region(editor, 2, "Change this button")
            # Reopen the first badge, save an edit, undo/redo it, then cancel another edit.
            run("xdotool", "mousemove", "--window", editor, "413", "170", "click", "1")
            dialog = window("Region 1")
            run("xdotool", "windowfocus", "--sync", dialog)
            key("ctrl+a")
            run("xdotool", "type", "--clearmodifiers", "Fix the revised header")
            key("Return")
            run("xdotool", "windowfocus", "--sync", editor)
            key("ctrl+z", "ctrl+y")
            run("xdotool", "mousemove", "--window", editor, "413", "170", "click", "1")
            dialog = window("Region 1")
            run("xdotool", "windowfocus", "--sync", dialog)
            key("ctrl+a")
            run("xdotool", "type", "--clearmodifiers", "Discard this edit")
            key("Escape")

            run("xdotool", "windowfocus", "--sync", editor)
            key("ctrl+z", "ctrl+y")
            key("Delete", "ctrl+z")  # Clear and restore both regions.
            drag(editor, 400, 150)
            dialog = window("Region 3")
            run("xdotool", "windowfocus", "--sync", dialog)
            key("Escape")  # Cancel must not consume a number.
            # Space-drag must pan without opening a comment or consuming a number.
            run("xdotool", "windowfocus", "--sync", editor)
            run("xdotool", "keydown", "space")
            time.sleep(0.2)
            drag(editor, 400, 300)
            run("xdotool", "keyup", "space")
            if os.environ.get("ROCKS_TEST_RIGHT_CROP"):
                drag(editor, 300, 80, button=3, dx=500, dy=350)
                key("Escape")  # Dismiss the crop without closing the editor.
            key("z")  # Arrow shares the region comment/number lifecycle.
            if os.environ.get("ROCKS_TEST_RIGHT_CROP"):
                drag(editor, 300, 80, button=3, dx=500, dy=350)
            region(editor, 3, "Third comment")
            run("xdotool", "windowfocus", "--sync", editor)
            key("Return")
            assert process.wait(timeout=15) == 0
            text = run("xclip", "-selection", "clipboard", "-o", "-target", "text/plain;charset=utf-8").decode()
            assert text == "1. Fix the revised header\n2. Change this button\n3. Third comment", repr(text)
            result = run("xclip", "-selection", "clipboard", "-o", "-target", "image/png")
            assert result.startswith(b"\x89PNG") and result != png
            expected_size = os.environ.get("ROCKS_EXPECT_SIZE", "800,600")
            dimensions = struct.unpack(">II", result[16:24])
            if os.environ.get("ROCKS_TEST_RIGHT_CROP"):
                assert 0 < dimensions[0] < 800 and 0 < dimensions[1] < 600, dimensions
            else:
                assert dimensions == tuple(map(int, expected_size.split(",")))
            html = run("xclip", "-selection", "clipboard", "-o", "-target", "text/html").decode()
            assert 'data-rocks="regions"' in html and 'data-searching-satty="regions"' in html and "Third comment" in html
            # AI chats attach the PNG and a Markdown file with the comments.
            uris = run("xclip", "-selection", "clipboard", "-o", "-target", "text/uri-list").decode().split()
            assert [Path(unquote(urlparse(uri).path)).suffix for uri in uris] == [".png", ".md"], uris
            exported = [Path(unquote(urlparse(uri).path)) for uri in uris]
            assert exported[0].read_bytes() == result
            assert exported[1].read_text().endswith(text + "\n"), exported[1].read_text()
            gnome = run("xclip", "-selection", "clipboard", "-o", "-target", "x-special/gnome-copied-files").decode()
            assert gnome.splitlines() == ["copy", *uris], gnome
            Path("/tmp/rocks-workflow-result.png").write_bytes(result)
            # Replacing the clipboard releases the background owner.
            subprocess.run(["xclip", "-selection", "clipboard"], input=b"test complete", check=True)
            # A plain capture also survives early-exit without relying on wl-copy.
            process = subprocess.Popen([binary, "--filename", str(image),
                "--actions-on-enter", "save-to-clipboard", "--early-exit", "copy",
                "--disable-notifications", "--title", "rocks Plain Test"],
                env=env, stdout=log, stderr=log)
            editor = window("^rocks Plain Test$")
            run("xdotool", "windowfocus", "--sync", editor)
            key("Return")
            assert process.wait(timeout=15) == 0
            plain = run("xclip", "-selection", "clipboard", "-o", "-target", "image/png")
            assert struct.unpack(">II", plain[16:24]) == (800, 600)
            subprocess.run(["xclip", "-selection", "clipboard"], input=b"test complete", check=True)
            print("PASS: numbered regions, edits, undo/redo, cancellation, PNG/text/HTML/files and plain capture after exit")
        except BaseException:
            log.flush()
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
