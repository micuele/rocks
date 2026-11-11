# Local Satty fork

`upstream/` is a local fork based on the Satty v0.22.0 source snapshot from
https://github.com/Satty-org/Satty, commit
`9f99065e4177820e965f5475503e5c3bdbd74e6a`, licensed under MPL-2.0
(see `upstream/LICENSE`). It is vendored into this repository, without a nested
Git repository or a hosted GitHub fork. Local changes add region comments and
clipboard export; upstream notices and licensing are preserved.

On this KDE Wayland computer, Print Screen launches `screenshot.sh`: Spectacle
captures the entire desktop into a temporary file, then Satty opens fullscreen
with the rectangle tool selected and Small annotation size. Hold Space and drag
to pan the image. The editor uses borderless toolbar icons; the compact comment dialog matches
the searching.rocks image viewer, with a plain text field and keyboard hints. Drag a region, type its comment, and press
Enter to add it (Shift+Enter adds a new line). Escape in the comment dialog
cancels that region. Click a numbered badge to edit its existing comment; Enter saves,
Escape keeps the original, and undo/redo includes comment edits. Repeat for more regions, then press Enter in the editor
to copy and close. Escape in the editor cancels the screenshot.
On export, if visible annotations are confined to one monitor, only that entire
monitor is copied or saved. Annotations on multiple monitors (or no annotations)
keep the full desktop. A manual crop takes priority. The launcher reads monitor
geometry through `kscreen-doctor`; unavailable or mismatched geometry keeps the
full capture. Undo/redo and clearing/restoring annotations keep comments and numbers together.
Temporary captures are removed on exit.

Boxes and arrows both prompt for a comment and share sequential numbers.
The image contains numbered annotations. The same clipboard selection also offers
numbered plain text and HTML containing the image and comments. Destination apps
choose which formats to paste; searching.rocks recognizes this payload and pastes
both the image attachment and comments. Other plain text fields receive comments;
image editors receive the image. Rich text support depends on the destination.
The clipboard owner stays alive until another copy replaces the selection.
On KDE Wayland it uses KDE's XWayland clipboard bridge (requires `DISPLAY`), so
the background owner can publish without taking keyboard focus.

The local launcher uses `~/.local/bin/satty`, initially installed from the matching
upstream x86_64 release archive (SHA-256
`eb7a028c4a5ce331c2f355add8e2b807a7d697fe4495f7e1965786a4f6bcd5b8`).
After modifying the fork, build with `cargo build --release --locked` inside
`upstream/` and install `target/release/satty` to that same path. Building needs
Rust plus GTK4, libadwaita, epoxy and fontconfig development packages; see upstream's
README for details. This host uses the isolated rootless Podman container
`searching-satty-build`, with this directory mounted at `/work`; no compiler was
added to the immutable host. Rebuild using:

```sh
podman start searching-satty-build
podman exec searching-satty-build sh -c 'cd /work/upstream && /root/.cargo/bin/cargo build --release --locked'
/usr/bin/python3 tools/satty/install-kde.py tools/satty/upstream/target/release/satty
```

Verify with `cargo test --release --locked` and `cargo fmt --check` inside
`upstream/`. `test-workflow.py` exercises three regions, undo/redo, clear/restore,
cancellation and PNG/text/HTML clipboard availability after the editor exits.
Run it on an isolated display (needs Xvfb, xdotool, xclip and software OpenGL):

```sh
podman exec searching-satty-build sh -c 'cd /work && xvfb-run -a -s "-screen 0 1280x900x24" python3 test-workflow.py upstream/target/release/satty'
```

To check automatic cropping through the same clipboard workflow, set
`SEARCHING_SATTY_MONITORS='0,0,20,600;20,0,780,600'` and
`SATTY_EXPECT_SIZE='780,600'` before `xvfb-run` in that command. The simulated
layout keeps all workflow annotations on its second monitor.

Stop the build container when finished with `podman stop searching-satty-build`.

`install-kde.py /path/to/satty` installs the binary and user application launcher,
and moves the existing Print shortcut from Spectacle to this launcher using
KGlobalAccel. Other Spectacle shortcuts stay available. Run it with the system
Python (`/usr/bin/python3`), which provides the `dbus` module.
`install-kde.py --restore` restores the saved shortcuts and removes this launcher's
shortcut. The original bindings are saved once in
`~/.local/share/searching-satty/shortcuts.json`.

The desktop launcher points to this checkout; moving the repository requires
rerunning the installer. Electron's build and runtime do not depend on Satty.
