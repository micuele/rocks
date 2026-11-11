#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Install a built rocks editor and its launchers, per user or into a package tree.

Without --prefix this installs for the current user under ~/.local. Packages use
`--prefix /usr --destdir STAGING`, which only writes files and never touches the
running system (no icon cache updates, no shortcut changes).
"""
import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
APP = "rocks.searching.Rocks"
# Installed next to capture.py; kde_shortcut.py backs rocks-setup-kde.
SHARED = ("capture.py", "config.toml", "kde_shortcut.py")


def quoted(value):
    # freedesktop Exec syntax, not shell syntax.
    return '"' + str(value).replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%') + '"'


def write_launcher(path, python, script, editor):
    # The editor path is baked in so a user and a system install never mix;
    # ROCKS_EDITOR still overrides it for development.
    path.write_text(
        "#!/bin/sh\n"
        f"ROCKS_EDITOR=\"${{ROCKS_EDITOR:-{editor}}}\"\nexport ROCKS_EDITOR\n"
        f"exec {python} {shlex.quote(str(script))} \"$@\"\n"
    )
    path.chmod(0o755)


def copy_file(source, target, mode=0o644):
    target.parent.mkdir(parents=True, exist_ok=True)
    # Replace atomically so a running rocks is never left with a partial binary.
    with tempfile.NamedTemporaryFile(dir=target.parent, delete=False) as staging:
        temporary = Path(staging.name)
    try:
        shutil.copyfile(source, temporary)
        temporary.chmod(mode)
        temporary.replace(target)
    finally:
        temporary.unlink(missing_ok=True)


def install(binary, prefix=None, destdir=None):
    if not sys.platform.startswith("linux"):
        raise SystemExit("rocks currently supports Linux only.")
    binary = Path(binary).resolve(strict=True)
    subprocess.run([str(binary), "--version"], check=True, stdout=subprocess.DEVNULL)
    if prefix is None:
        if destdir:
            raise SystemExit("--destdir requires --prefix.")
        system = False
        data = Path(os.environ.get("XDG_DATA_HOME", str(Path.home()/".local/share")))
        bindir = Path.home()/".local/bin"
        python = "python3"
        licenses = data/"rocks"
    else:
        system = True
        prefix = Path(prefix)
        if not prefix.is_absolute():
            raise SystemExit("--prefix must be absolute.")
        data = prefix/"share"
        bindir = prefix/"bin"
        python = "/usr/bin/python3"
        licenses = data/"licenses/rocks"
    # Paths written into launchers and .desktop are final paths; files go under destdir.
    stage = (lambda path: Path(destdir)/path.relative_to("/")) if destdir else (lambda path: path)
    share = data/"rocks"
    stage(share).mkdir(parents=True, exist_ok=True)
    stage(bindir).mkdir(parents=True, exist_ok=True)
    editor = bindir/"rocks"
    if binary != stage(editor):
        copy_file(binary, stage(editor), 0o755)
    for name in SHARED:
        copy_file(ROOT/name, stage(share/name))
    for name in ("LICENSE", "NOTICE.md"):
        copy_file(ROOT/name, stage(licenses/name))
    launcher = bindir/"rocks-capture"
    # Launchers and all their resources are installed, never tied to this checkout.
    write_launcher(stage(launcher), python, share/"capture.py", editor)
    write_launcher(stage(bindir/"rocks-setup-kde"), "/usr/bin/python3", share/"kde_shortcut.py", editor)
    copy_file(ROOT/"assets/rocks.png", stage(data/f"icons/hicolor/256x256/apps/{APP}.png"))
    desktop = stage(data/f"applications/{APP}.desktop")
    desktop.parent.mkdir(parents=True, exist_ok=True)
    desktop.write_text(
        "[Desktop Entry]\nType=Application\nName=rocks\nGenericName=Screenshot Annotation\n"
        "Comment=Capture the screen and annotate numbered regions\n"
        f"Exec={quoted(launcher)}\nIcon={APP}\nStartupWMClass={APP}\n"
        "Terminal=false\nCategories=Utility;Graphics;\nStartupNotify=false\n"
    )
    desktop.chmod(0o644)
    if system:
        print(f"Staged rocks for {prefix} in {destdir or '/'}.")
        return
    if shutil.which("gtk-update-icon-cache"):
        subprocess.run(["gtk-update-icon-cache", "--force", "--ignore-theme-index", str(data/"icons/hicolor")], check=False)
    print(f"Installed rocks to {bindir}. Launch rocks-capture or choose rocks in the application menu.")
    print("When distributing this binary, provide recipients the matching MPL-covered source and a source location with the package.")


def main(argv=None):
    parser = argparse.ArgumentParser(description="Install a built rocks binary.")
    parser.add_argument("binary", help="path to the built rocks executable")
    parser.add_argument("--prefix", help="system install prefix such as /usr (default: per-user ~/.local)")
    parser.add_argument("--destdir", help="stage a --prefix install under this directory (for packaging)")
    args = parser.parse_args(argv)
    install(args.binary, args.prefix, args.destdir)


if __name__ == "__main__":
    main()
