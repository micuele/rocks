# Usage details

## Capture

`rocks-capture` takes a screenshot with your desktop's tool and opens it in the
editor. It picks a backend automatically, or you can choose one:

```sh
rocks-capture                             # pick a backend for this desktop
rocks-capture --backend spectacle         # KDE (X11 or Wayland)
rocks-capture --backend grim              # wlroots Wayland (Sway, Hyprland)
rocks-capture --backend gnome-screenshot  # X11
rocks-capture --backend import            # X11, ImageMagick
rocks --filename /path/to/image.png       # edit an existing image
```

- **KDE** is the tested workflow. The other backends are covered by unit tests
  but not yet tried on their desktops.
- **GNOME on Wayland** is not integrated: take the screenshot with GNOME's own
  tool and open the file with `rocks --filename`.
- **wlroots** capture needs the compositor's screencopy support.
- **Multi-monitor:** with Spectacle, rocks uses KDE's optional `kscreen-doctor`
  to crop the export to the monitor you worked on. Other backends keep the full
  capture. A manual crop always wins.
- Temporary captures are deleted when the editor exits, and a per-user lock
  stops two captures running at once.
- On desktops other than KDE, bind Print Screen to `rocks-capture` in the
  desktop's shortcut settings.

## Clipboard

Copying offers several formats together, and each application picks the one it
supports:

- the annotated **PNG**
- the comments as **numbered plain text**
- **HTML** with the image and comments (marked `data-rocks="regions"`, plus the
  older `data-searching-satty="regions"` marker for existing searching.rocks
  installs)
- when there are comments, **two files** — the PNG and a Markdown file with the
  numbered comments — as `text/uri-list` and `x-special/gnome-copied-files`.
  AI chats that accept pasted files attach both, so comments are read as text.

The files are written to `$XDG_CACHE_HOME/rocks/clipboard` (default
`~/.cache/rocks/clipboard`). Files older than a day are removed on the next copy.
Sandboxed (Flatpak or Snap) browsers may not be able to read that directory.

A small background process keeps the clipboard available after the editor
closes, until something else is copied. On Wayland it needs XWayland
(`DISPLAY`); pure Wayland sessions without XWayland are not supported.

## Configuration

The editor reads `$XDG_CONFIG_HOME/rocks/config.toml` by default;
`rocks-capture` passes its installed config explicitly. The `[general]` section
accepts `fullscreen`, `initial-tool`, `corner-roundness`, `early-exit`,
`actions-on-enter`, `actions-on-escape`, `output-filename`,
`disable-notifications` and `title` (see `rocks --help`). Unknown keys are
rejected.

| Environment variable | Purpose |
|---|---|
| `ROCKS_MONITORS` | Monitor rectangles `x,y,width,height;…` in capture pixels |
| `ROCKS_EDITOR` | Editor executable used by `rocks-capture` (for development) |

## What gets installed

| | Packages | `install.py` (per user) |
|---|---|---|
| Programs | `/usr/bin/rocks`, `rocks-capture`, `rocks-setup-kde` | `~/.local/bin/…` |
| Capture script and config | `/usr/share/rocks/` | `$XDG_DATA_HOME/rocks/` |
| Licenses | `/usr/share/licenses/rocks/` | `$XDG_DATA_HOME/rocks/` |
| Menu entry and icon | `/usr/share/applications`, `/usr/share/icons` | `$XDG_DATA_HOME/…` |

Packages don't update themselves; install the next release's package to
upgrade. A per-user install copies everything it needs, so moving or deleting
the checkout doesn't break it; reinstall after changing code or config.

`rocks-setup-kde` (and `install-kde.py`) uses the system Python's `dbus` module
(`python3-dbus` / `python-dbus`). It takes Print Screen from Spectacle, keeps
Spectacle's other shortcuts, migrates the old searching.rocks shortcut, and
saves the original bindings so `--restore` can put them back.
