#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
# Build a .deb from an already built rocks binary. Run from any directory on a
# Debian/Ubuntu system with dpkg-dev; build on the oldest release you support
# (Ubuntu 22.04) so the package installs on newer ones too.
#   packaging/deb/build-deb.sh target/release/rocks [output-directory]
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
binary=$(realpath "$1")
out=$(realpath "${2:-.}")
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
arch=$(dpkg --print-architecture)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
tree="$work/tree"

python3 "$root/install.py" --prefix /usr --destdir "$tree" "$binary"
install -Dm644 "$root/LICENSE" "$tree/usr/share/doc/rocks/copyright"
install -Dm644 "$root/NOTICE.md" "$tree/usr/share/doc/rocks/NOTICE.md"

# Library dependencies come from the binary itself, so they always match the build system.
mkdir -p "$work/debian"
printf 'Source: rocks\n\nPackage: rocks\nArchitecture: any\n' > "$work/debian/control"
libs=$(cd "$work" && dpkg-shlibdeps -O "$tree/usr/bin/rocks" | sed 's/^shlibs:Depends=//')

mkdir -p "$tree/DEBIAN"
cat > "$tree/DEBIAN/control" <<EOF
Package: rocks
Version: $version
Architecture: $arch
Maintainer: Michele Qirici <51047047+micuele@users.noreply.github.com>
Installed-Size: $(du -sk --exclude=DEBIAN "$tree" | cut -f1)
Depends: $libs, python3
Recommends: hicolor-icon-theme
Suggests: kde-spectacle | grim | gnome-screenshot | imagemagick, python3-dbus
Section: graphics
Priority: optional
Homepage: https://github.com/micuele/rocks
Description: Screenshot annotation with numbered comments
 rocks captures the screen and lets you mark rectangles and arrows with
 numbered comments, then copies the image, the numbered text and HTML
 together. rocks-capture needs Spectacle (KDE), grim (wlroots) or
 gnome-screenshot/ImageMagick (X11). Run rocks-setup-kde to bind Print Screen.
EOF
(cd "$tree" && find usr -type f -exec md5sum {} + > DEBIAN/md5sums)

mkdir -p "$out"
package="$out/rocks_${version}_${arch}.deb"
dpkg-deb --root-owner-group -Zxz --build "$tree" "$package" >/dev/null
echo "$package"
