#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Linux capture adapters. The GTK editor only needs a PNG filename."""
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import time

BACKENDS = ("spectacle", "grim", "gnome-screenshot", "import")


def choose_backend(requested, environ, available=shutil.which):
    if requested != "auto":
        if not available(requested):
            raise RuntimeError(f"Missing capture command: {requested}")
        return requested
    desktop = environ.get("XDG_CURRENT_DESKTOP", "").lower()
    wayland = bool(environ.get("WAYLAND_DISPLAY"))
    candidates = []
    if "kde" in desktop:
        candidates.append("spectacle")
    if wayland:
        candidates.append("grim")
    else:
        candidates.extend(("gnome-screenshot", "import", "spectacle"))
    for backend in candidates:
        if available(backend):
            return backend
    raise RuntimeError("No supported capture backend found. Install Spectacle on KDE, grim on wlroots Wayland, or gnome-screenshot/ImageMagick on X11. You can also open an existing image with rocks --filename IMAGE.")


def capture_command(backend, filename):
    return {
        "spectacle": ["spectacle", "--background", "--nonotify", "--fullscreen", "--output", str(filename)],
        "grim": ["grim", "-t", "png", str(filename)],
        "gnome-screenshot": ["gnome-screenshot", "--file", str(filename)],
        "import": ["import", "-window", "root", str(filename)],
    }[backend]


def wait_png(filename, seconds=15):
    # Spectacle can forward to an existing process before it finishes saving.
    deadline = time.monotonic() + seconds
    while True:
        try:
            with filename.open("rb") as stream:
                if stream.read(8) == b"\x89PNG\r\n\x1a\n":
                    stream.seek(-12, 2)
                    if stream.read() == b"\0\0\0\0IEND\xaeB`\x82":
                        return
        except (OSError, ValueError):
            pass
        if time.monotonic() >= deadline:
            raise RuntimeError("Capture did not produce a complete PNG.")
        time.sleep(0.1)


def kde_monitors(filename):
    """Map KDE logical geometry to capture pixels; unknown layouts keep all pixels."""
    try:
        data = json.loads(subprocess.check_output(["kscreen-doctor", "-j"], timeout=3))
        outputs = [o for o in data["outputs"] if o["enabled"] and o["connected"]]
        left = min(o["pos"]["x"] for o in outputs)
        top = min(o["pos"]["y"] for o in outputs)
        right = max(o["pos"]["x"] + o["size"]["width"] for o in outputs)
        bottom = max(o["pos"]["y"] + o["size"]["height"] for o in outputs)
        with filename.open("rb") as image:
            image.seek(16)
            width, height = struct.unpack(">II", image.read(8))
        sx, sy = width / (right-left), height / (bottom-top)
        if abs(sx-sy) > 0.01:
            return ""
        rects = []
        for o in outputs:
            x, y = o["pos"]["x"]-left, o["pos"]["y"]-top
            x1, y1 = round(x*sx), round(y*sy)
            x2, y2 = round((x+o["size"]["width"])*sx), round((y+o["size"]["height"])*sy)
            rects.append(f"{x1},{y1},{x2-x1},{y2-y1}")
        return ";".join(dict.fromkeys(rects))
    except (OSError, ValueError, KeyError, TypeError, struct.error, ZeroDivisionError, subprocess.SubprocessError):
        return ""


def main():
    if not sys.platform.startswith("linux"):
        raise RuntimeError("rocks currently supports Linux only.")
    import fcntl
    parser = argparse.ArgumentParser(prog="rocks-capture", description="Capture the desktop and annotate it with rocks.")
    parser.add_argument("--backend", choices=("auto", *BACKENDS), default="auto")
    args = parser.parse_args()
    backend = choose_backend(args.backend, os.environ)
    runtime = Path(os.environ.get("XDG_RUNTIME_DIR", str(Path.home()/".cache"))) / "rocks"
    runtime.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (runtime / "capture.lock").open("w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return
        with tempfile.TemporaryDirectory(prefix="capture-", dir=runtime) as directory:
            filename = Path(directory) / "screen.png"
            result = subprocess.run(capture_command(backend, filename), timeout=30)
            if result.returncode and backend != "spectacle":
                raise RuntimeError(f"{backend} failed or capture was cancelled.")
            wait_png(filename)
            env = dict(os.environ)
            env["ROCKS_MONITORS"] = kde_monitors(filename) if backend == "spectacle" else ""
            editor = os.environ.get("ROCKS_EDITOR", str(Path.home()/".local/bin/rocks"))
            subprocess.run([editor, "--config", str(Path(__file__).resolve().with_name("config.toml")), "--filename", str(filename)], env=env, check=True)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(f"rocks: {error}", file=sys.stderr)
        sys.exit(1)
