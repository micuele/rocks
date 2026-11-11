#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Install a built rocks editor and an independent launcher for this Linux user."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
APP = "rocks.searching.Rocks"


def quoted(value):
    # freedesktop Exec syntax, not shell syntax.
    return '"' + str(value).replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%') + '"'


def install(binary):
    if not sys.platform.startswith("linux"):
        raise SystemExit("rocks currently supports Linux only.")
    binary = Path(binary).resolve(strict=True)
    subprocess.run([str(binary), "--version"], check=True)
    data = Path(os.environ.get("XDG_DATA_HOME", str(Path.home()/".local/share")))
    share = data / "rocks"
    bindir = Path.home()/".local/bin"
    share.mkdir(parents=True, exist_ok=True)
    bindir.mkdir(parents=True, exist_ok=True)
    target = bindir/"rocks"
    if binary != target:
        with tempfile.NamedTemporaryFile(dir=bindir, delete=False) as staging:
            temporary = Path(staging.name)
        try:
            shutil.copy2(binary, temporary)
            temporary.chmod(0o755)
            temporary.replace(target)
        finally:
            temporary.unlink(missing_ok=True)
    for name in ("capture.py", "config.toml", "LICENSE", "NOTICE.md"):
        shutil.copy2(ROOT/name, share/name)
    launcher = bindir/"rocks-capture"
    # The launcher and all its resources are installed, never tied to this checkout.
    import shlex
    launcher.write_text('#!/bin/sh\nexec python3 ' + shlex.quote(str(share/"capture.py")) + ' "$@"\n')
    launcher.chmod(0o755)
    icons = data/"icons/hicolor/256x256/apps"
    icons.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT/"assets/rocks.png", icons/f"{APP}.png")
    applications = data/"applications"
    applications.mkdir(parents=True, exist_ok=True)
    (applications/f"{APP}.desktop").write_text(
        "[Desktop Entry]\nType=Application\nName=rocks\nGenericName=Screenshot Annotation\n"
        f"Exec={quoted(launcher)}\nIcon={APP}\nStartupWMClass={APP}\n"
        "Terminal=false\nCategories=Utility;Graphics;\nStartupNotify=false\n"
    )
    if shutil.which("gtk-update-icon-cache"):
        subprocess.run(["gtk-update-icon-cache", "--force", "--ignore-theme-index", str(data/"icons/hicolor")], check=False)
    print(f"Installed rocks to {bindir}. Launch rocks-capture or choose rocks in the application menu.")
    print("When distributing this binary, provide recipients the matching MPL-covered source and a source location with the package.")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 install.py /path/to/rocks")
    install(sys.argv[1])
