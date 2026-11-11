# rocks

A Linux screenshot editor with numbered regions and comments, using the same
stone icon as searching.rocks. This is an independent Rust/GTK application,
forked from Satty v0.22.0 under MPL-2.0. See [NOTICE.md](NOTICE.md) and [LICENSE](LICENSE).
It has no searching.rocks, Electron, database or Node.js dependency.

## Build and install

Install Rust and development packages for GTK4, libadwaita, epoxy and fontconfig,
plus a C compiler, pkg-config and the OpenGL runtime. On Fedora the development
packages are `gtk4-devel libadwaita-devel libepoxy-devel fontconfig-devel gcc
pkgconf-pkg-config`; on Debian-based systems they are `libgtk-4-dev
libadwaita-1-dev libepoxy-dev libfontconfig1-dev build-essential pkg-config`.
A recent distribution is needed for this GTK stack.

```sh
cargo build --release --locked
python3 install.py target/release/rocks
```

This installs the editor as `~/.local/bin/rocks`, the capture command as
`~/.local/bin/rocks-capture`, and the config, capture script, license and notice
under `$XDG_DATA_HOME/rocks` (default `~/.local/share/rocks`). The application
menu entry is **rocks**, using the stone icon. Add `~/.local/bin` to PATH for
terminal use. Installation copies everything needed; moving this checkout does
not break the installed application. Reinstall after changing the code/config.

On KDE, install and assign Print Screen with the system Python and its `dbus`
module:

```sh
/usr/bin/python3 install-kde.py target/release/rocks
# Restore the original Print Screen binding later:
/usr/bin/python3 install-kde.py --restore
```

The KDE installer migrates the previous searching.rocks/Satty shortcut, retaining
its original Spectacle bindings for restoration. Other Spectacle shortcuts are
preserved. The previous `~/.local/bin/satty` binary is left intact for other uses.
On other desktops, bind Print Screen to the absolute path of `rocks-capture`
using that desktop's shortcut settings.

## Capture support

Linux is the only supported platform. The editor accepts PNG files independently
of the desktop capture method:

```sh
rocks-capture                       # choose a backend for the current desktop
rocks-capture --backend spectacle   # KDE (X11 or Wayland)
rocks-capture --backend grim        # wlroots Wayland, e.g. Sway
rocks-capture --backend gnome-screenshot  # X11
rocks-capture --backend import      # X11, ImageMagick
rocks --filename /path/to/image.png # edit an existing image
```

Install the corresponding capture command. KDE is the existing validated host
workflow. Other adapters have command/selection tests but still need validation
on their target desktops. GNOME Wayland capture is not currently integrated:
use its screenshot UI and open the resulting file. wlroots capture requires the
compositor's screencopy support. Wayland clipboard export currently requires
XWayland (`DISPLAY`) for the background clipboard owner; pure Wayland sessions
without XWayland are not supported by this clipboard workflow.

Capture adapters are isolated in `capture.py`. Adding another desktop capture
method does not change the editor. Automatic single-monitor export uses KDE's
optional `kscreen-doctor`; other backends keep the full capture. A manual crop
takes priority. Temporary captures are deleted after the editor exits, and a
per-user lock prevents concurrent captures.

## Annotate and copy

Capture opens fullscreen with the rectangle tool selected. Drag a rectangle or
arrow, type a comment, and press Enter (Shift+Enter adds a newline). Escape in
the comment dialog cancels that region. Click a numbered badge to edit its
comment. Right-drag to crop without changing the selected drawing tool; right-drag
the crop interior or its handles to move or resize it. Escape clears a crop before
exiting. Undo/redo keeps the comments and numbers together. Hold Space and drag
to pan. Enter in the editor copies and closes; Escape cancels.
Tool shortcuts: B brush, I line, Z arrow, R rectangle. Ctrl+Z/Ctrl+Y undo/redo,
Ctrl+C copies, Ctrl+S/Ctrl+Shift+S save, Delete clears all annotations.

The clipboard offers PNG, numbered plain text and HTML together. searching.rocks
pastes the image and comments; other applications choose their supported format.
When regions have comments, the clipboard also offers two files, like a file
manager copy (`text/uri-list` and `x-special/gnome-copied-files`): the PNG and a
Markdown file with the numbered comments, written to
`$XDG_CACHE_HOME/rocks/clipboard` (default `~/.cache/rocks/clipboard`). AI chats
that accept pasted files attach both, so comments are read as text rather than
from the image. Exported files older than one day are removed on the next copy.
Sandboxed (Flatpak/Snap) browsers may not be able to read that directory.
The clipboard owner stays alive until another copy replaces it. Both the new
`data-rocks="regions"` HTML marker and legacy `data-searching-satty="regions"`
marker are exported for compatibility with existing searching.rocks installs.
The editor uses `$XDG_CONFIG_HOME/rocks/config.toml` by default; rocks-capture
passes its installed config explicitly.
The config's `[general]` section accepts `fullscreen`, `initial-tool`, `corner-roundness`,
`early-exit`, `actions-on-enter`, `actions-on-escape`, `output-filename`,
`disable-notifications` and `title` (see `rocks --help`); other upstream Satty
keys are rejected. `ROCKS_MONITORS` supplies optional monitor
rectangles as `x,y,width,height;…` in capture pixels. `ROCKS_EDITOR` overrides the
editor executable for capture development.

## Verify

```sh
cargo fmt --check
cargo test --release --locked
python3 -m unittest discover -s tests -v
# Needs Xvfb, xdotool, xclip and software OpenGL:
xvfb-run -a -s '-screen 0 1280x900x24' python3 test-workflow.py target/release/rocks
# Exercise right-drag cropping while preserving the drawing tool:
ROCKS_TEST_RIGHT_CROP=1 \
  xvfb-run -a -s '-screen 0 1280x900x24' python3 test-workflow.py target/release/rocks
# Exercise automatic monitor cropping:
ROCKS_MONITORS='0,0,20,600;20,0,780,600' ROCKS_EXPECT_SIZE='780,600' \
  xvfb-run -a -s '-screen 0 1280x900x24' python3 test-workflow.py target/release/rocks
```

On the original immutable Fedora host, the local `rocks-build` Podman container
has this repository mounted at `/work` and the existing Rust/GTK toolchain:

```sh
podman start rocks-build
podman exec rocks-build sh -c 'cd /work && /root/.cargo/bin/cargo build --release --locked'
podman exec rocks-build sh -c 'cd /work && /root/.cargo/bin/cargo test --release --locked && /root/.cargo/bin/cargo fmt --check'
podman exec rocks-build sh -c 'cd /work && xvfb-run -a -s "-screen 0 1280x900x24" python3 test-workflow.py target/release/rocks'
podman stop rocks-build
```

The container is a local convenience, not a prerequisite or published image.
The source is hosted at https://github.com/micuele/rocks (private for now).
Release distribution must include the notices and tell recipients how to obtain
the matching modified source; see [NOTICE.md](NOTICE.md). No macOS/Windows support,
signing or automatic updates are provided.
