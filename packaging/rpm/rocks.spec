# SPDX-License-Identifier: MPL-2.0
# Release tarball build:  rpmbuild -bb rocks.spec  (with the v%{version} tarball in SOURCES)
# Checkout build (CI):    rpmbuild -bb --build-in-place packaging/rpm/rocks.spec
# Cargo downloads crates, so the build needs network access.
%global debug_package %{nil}

Name:           rocks
Version:        0.1.0
Release:        1%{?dist}
Summary:        Screenshot annotation with numbered comments
License:        MPL-2.0
URL:            https://github.com/micuele/rocks
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  python3
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  pkgconfig(epoxy)
BuildRequires:  pkgconfig(fontconfig)
# Library requirements are generated from the binary.
Requires:       python3
Requires:       hicolor-icon-theme
Suggests:       spectacle
Suggests:       grim
Suggests:       gnome-screenshot
Suggests:       ImageMagick
Suggests:       python3-dbus

%description
rocks captures the screen and lets you mark rectangles and arrows with
numbered comments, then copies the image, the numbered text and HTML together.
rocks-capture needs Spectacle (KDE), grim (wlroots) or gnome-screenshot or
ImageMagick (X11). Run rocks-setup-kde to bind Print Screen on KDE.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked

%install
python3 install.py --prefix %{_prefix} --destdir %{buildroot} target/release/rocks

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%license %{_datadir}/licenses/%{name}/NOTICE.md
%{_bindir}/rocks
%{_bindir}/rocks-capture
%{_bindir}/rocks-setup-kde
%{_datadir}/rocks/
%{_datadir}/applications/rocks.searching.Rocks.desktop
%{_datadir}/icons/hicolor/256x256/apps/rocks.searching.Rocks.png
