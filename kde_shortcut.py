#!/usr/bin/python3
# SPDX-License-Identifier: MPL-2.0
"""Assign Print Screen to rocks for the current KDE user, or restore Spectacle.

Installed as `rocks-setup-kde`. It only changes this user's KDE shortcuts and
needs the system Python's `dbus` module (python3-dbus / python-dbus).
"""
import json
import os
from pathlib import Path
import subprocess
import sys

APP = "rocks.searching.Rocks"
HOME = Path.home()
DATA = Path(os.environ.get("XDG_DATA_HOME", str(HOME/".local/share")))
ACTION = [f"{APP}.desktop", "_launch", "rocks", "Launch rocks"]
LEGACY_ACTION = ["rocks.searching.screenshot.desktop", "_launch", "Satty Screenshot", "Launch Satty Screenshot"]
STATE = DATA/"rocks/shortcuts.json"
LEGACY_STATE = HOME/".local/share/searching-satty/shortcuts.json"
PRINT_KEY = 0x01000009


def accelerator():
    try:
        import dbus
    except ImportError:
        raise SystemExit("rocks-setup-kde needs the system Python dbus module (python3-dbus or python-dbus).")
    bus = dbus.SessionBus()
    return dbus, dbus.Interface(bus.get_object("org.kde.kglobalaccel", "/kglobalaccel"), "org.kde.KGlobalAccel")


def restore():
    dbus, accel = accelerator()
    saved = json.loads((STATE if STATE.exists() else LEGACY_STATE).read_text())
    for action in (ACTION, LEGACY_ACTION):
        accel.setShortcut(action, dbus.Array([], signature="i"), 4)
    for item in saved:
        accel.setShortcut(item["action"], dbus.Array(item["keys"], signature="i"), 4)
    print("Original screenshot shortcuts restored; rocks remains installed.")


def assign():
    dbus, accel = accelerator()
    actions = accel.allActionsForComponent(["org.kde.spectacle.desktop", "", "", ""])
    saved = [{"action": list(action), "keys": list(accel.shortcut(action))}
             for action in actions if PRINT_KEY in accel.shortcut(action)]
    if not STATE.exists():
        STATE.parent.mkdir(parents=True, exist_ok=True)
        STATE.write_text(LEGACY_STATE.read_text() if LEGACY_STATE.exists() else json.dumps(saved, indent=2)+"\n")
    subprocess.run(["kbuildsycoca6"], check=True)
    for item in saved:
        accel.setShortcut(item["action"], dbus.Array([k for k in item["keys"] if k != PRINT_KEY], signature="i"), 4)
    accel.setShortcut(LEGACY_ACTION, dbus.Array([], signature="i"), 4)
    accel.doRegister(ACTION)
    keys = accel.setShortcut(ACTION, dbus.Array([PRINT_KEY], signature="i"), 4)
    if PRINT_KEY not in keys:
        raise SystemExit("KDE did not assign Print Screen. Restore with --restore.")
    (HOME/".local/share/applications/rocks.searching.screenshot.desktop").unlink(missing_ok=True)
    subprocess.run(["kbuildsycoca6"], check=True)
    print("Print Screen now launches rocks.")


def main(argv):
    if argv == ["--restore"]:
        restore()
    elif not argv:
        assign()
    else:
        raise SystemExit("Usage: rocks-setup-kde [--restore]")


if __name__ == "__main__":
    main(sys.argv[1:])
