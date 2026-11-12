# Third-party notices and source availability

rocks is a modified fork of Satty v0.22.0, based on source snapshot
`9f99065e4177820e965f5475503e5c3bdbd74e6a` from
https://github.com/Satty-org/Satty . Original authors include Matthias Gabriel
and the Satty contributors. Existing copyright and license notices are retained.
The Mozilla Public License 2.0 is included verbatim in `LICENSE`.

Local changes include numbered rectangle/arrow comments, comment editing and
undo/redo, combined PNG/text/HTML clipboard export, monitor-aware export,
UI styling and panning, and extraction as rocks with a new name, stone icon,
Linux capture adapters, user installation and KDE shortcut migration.

MPL-covered source files, including modifications, remain under MPL-2.0.
New Python integration code in this repository is also offered under MPL-2.0.
The stone icon is shared with searching.rocks; the MPL does not grant trademark
rights to the rocks, searching.rocks or Satty names or branding.

The corresponding source for rocks, including all modifications, is published
at https://github.com/micuele/rocks . Each release tag `vX.Y.Z` matches the
`.deb`, `.rpm` and Arch packages attached to that GitHub release. Upstream's
repository alone does not contain our modifications. Anyone redistributing a
rocks binary must make the matching source available to recipients (this
repository and tag, or a source archive) and include this notice and `LICENSE`.

Upstream Satty tools and options that rocks does not use (text, blur, highlight,
markers, ellipse, colour palette, shell completions and others) were removed.
`docs/previous-integration.md` records the
former searching.rocks integration and its old paths; use the root README now.
