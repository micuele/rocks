# Development

## Build and test

See the README for build dependencies. Then:

```sh
cargo fmt --check
cargo test --release --locked
python3 -m unittest discover -s tests -v
```

End-to-end editor tests drive the real app on a virtual X display. They need
Xvfb, xdotool, xclip and software OpenGL:

```sh
run() { xvfb-run -a -s '-screen 0 1280x900x24' python3 test-workflow.py target/release/rocks; }
run                                                           # regions, edits, undo, clipboard
ROCKS_TEST_RIGHT_CROP=1 run                                   # right-drag crop keeps the tool
ROCKS_MONITORS='0,0,20,600;20,0,780,600' ROCKS_EXPECT_SIZE='780,600' run   # monitor auto-crop
```

### Local build container

On the original immutable Fedora host, the `rocks-build` Podman container has
this repository mounted at `/work` and the Rust/GTK toolchain. It is a local
convenience, not a published image.

```sh
podman start rocks-build
podman exec rocks-build sh -c 'cd /work && /root/.cargo/bin/cargo build --release --locked'
podman exec rocks-build sh -c 'cd /work && /root/.cargo/bin/cargo test --release --locked && /root/.cargo/bin/cargo fmt --check'
podman exec rocks-build sh -c 'cd /work && xvfb-run -a -s "-screen 0 1280x900x24" python3 test-workflow.py target/release/rocks'
podman stop rocks-build
```

## CI

`.github/workflows/ci.yml` runs every check above on each push to `main` and on
pull requests, on Ubuntu 22.04 (the oldest supported GTK). The run also builds
the `.deb` and attaches it as an artifact.

## Packaging

Recipes live in `packaging/`. All three stage files with
`install.py --prefix /usr --destdir STAGING`, so the installed layout is
defined once and packaging never writes outside `STAGING`.

| Package | Recipe | Built on |
|---|---|---|
| `.deb` | `packaging/deb/build-deb.sh` | Ubuntu 22.04 |
| `.rpm` | `packaging/rpm/rocks.spec` | Fedora 43 |
| Arch | `packaging/arch/PKGBUILD` (builds the surrounding checkout) | Arch Linux |

Packages are built on the oldest supported release so they also install on newer ones.

## Releases

1. Set the new version in `Cargo.toml` (then run `cargo build` to update
   `Cargo.lock`), `packaging/arch/PKGBUILD` (`pkgver`, reset `pkgrel=1`) and
   `packaging/rpm/rocks.spec` (`Version`). A unit test fails if they differ.
2. Commit, tag and push: `git tag v0.2.0 && git push origin main v0.2.0`.

`.github/workflows/release.yml` then runs the tests, builds the three packages
in parallel, installs each one as a smoke test, and publishes a GitHub release
with the packages and `SHA256SUMS`. Running the Release workflow manually from
the Actions tab builds the same packages as artifacts without publishing.

Anyone distributing binaries must include `LICENSE` and `NOTICE.md` and tell
recipients where to get the matching source; see [NOTICE.md](../NOTICE.md).
