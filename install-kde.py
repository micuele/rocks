#!/usr/bin/python3
# SPDX-License-Identifier: MPL-2.0
"""Install rocks and migrate the current KDE user's Print Screen shortcut."""
import json
import os
from pathlib import Path
import subprocess
import sys

import dbus
from install import APP, install

HOME = Path.home()
DATA = Path(os.environ.get("XDG_DATA_HOME", str(HOME/".local/share")))
ACTION = [f"{APP}.desktop", "_launch", "rocks", "Launch rocks"]
LEGACY_ACTION = ["rocks.searching.screenshot.desktop", "_launch", "Satty Screenshot", "Launch Satty Screenshot"]
STATE = DATA/"rocks/shortcuts.json"
LEGACY_STATE = HOME/".local/share/searching-satty/shortcuts.json"


def main():
    bus = dbus.SessionBus()
    accel = dbus.Interface(bus.get_object("org.kde.kglobalaccel", "/kglobalaccel"), "org.kde.KGlobalAccel")
    if sys.argv[1:] == ["--restore"]:
        saved = json.loads((STATE if STATE.exists() else LEGACY_STATE).read_text())
        for action in (ACTION, LEGACY_ACTION):
            accel.setShortcut(action, dbus.Array([], signature="i"), 4)
        for item in saved:
            accel.setShortcut(item["action"], dbus.Array(item["keys"], signature="i"), 4)
        print("Original screenshot shortcuts restored; rocks remains installed.")
        return
    if len(sys.argv) != 2:
        raise SystemExit("Usage: /usr/bin/python3 install-kde.py /path/to/rocks | --restore")
    install(sys.argv[1])
    actions = accel.allActionsForComponent(["org.kde.spectacle.desktop", "", "", ""])
    print_key = 0x01000009
    saved = [{"action": list(action), "keys": list(accel.shortcut(action))}
             for action in actions if print_key in accel.shortcut(action)]
    if not STATE.exists():
        STATE.write_text(LEGACY_STATE.read_text() if LEGACY_STATE.exists() else json.dumps(saved, indent=2)+"\n")
    subprocess.run(["kbuildsycoca6"], check=True)
    for item in saved:
        accel.setShortcut(item["action"], dbus.Array([k for k in item["keys"] if k != print_key], signature="i"), 4)
    accel.setShortcut(LEGACY_ACTION, dbus.Array([], signature="i"), 4)
    accel.doRegister(ACTION)
    keys = accel.setShortcut(ACTION, dbus.Array([print_key], signature="i"), 4)
    if print_key not in keys:
        raise SystemExit("KDE did not assign Print Screen. Restore with --restore.")
    (HOME/".local/share/applications/rocks.searching.screenshot.desktop").unlink(missing_ok=True)
    subprocess.run(["kbuildsycoca6"], check=True)
    print("Print Screen now launches rocks.")


if __name__ == "__main__":
    main()
