#!/usr/bin/python3
# SPDX-License-Identifier: MPL-2.0
"""Install rocks for this user and migrate the KDE Print Screen shortcut."""
import sys

from install import install
import kde_shortcut


def main():
    if sys.argv[1:] == ["--restore"]:
        kde_shortcut.restore()
        return
    if len(sys.argv) != 2:
        raise SystemExit("Usage: /usr/bin/python3 install-kde.py /path/to/rocks | --restore")
    kde_shortcut.accelerator()  # Fail before installing if dbus is unavailable.
    install(sys.argv[1])
    kde_shortcut.assign()


if __name__ == "__main__":
    main()
