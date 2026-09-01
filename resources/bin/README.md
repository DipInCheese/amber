# Bundled sidecar binaries

`idevice_id`/`idevicebackup2` (from libimobiledevice) get built and placed
here fresh at release-build time — see `.github/workflows/release.yml` (and
`.github/workflows/build-sidecars.yml`, a `workflow_dispatch` job used to
validate the approach and iterate on it without cutting a release). This
directory is never committed to; `.gitignore` excludes everything here
except this README.

**Windows** (`x86_64-pc-windows-msvc`): `idevice_id`/`idevicebackup2` come
from MSYS2's `mingw-w64-x86_64-libimobiledevice` package (MSYS2 is
pre-installed on GitHub's `windows-latest` runners). The full DLL closure
is resolved by walking `ldd` output breadth-first and copied in flat
alongside the two executables - Windows' DLL search order checks the
executable's own directory first, so no relinking is needed. **Verified
for real**: downloaded the CI-built binaries and ran them standalone on a
machine with no MSYS2 installed at all, and built a full local installer
(`.msi`/`.exe`) that bundles them correctly (`tauri.conf.json`'s
`externalBin` + a generated `resources` map with flat targets - see
`release.yml` for exactly how).

**macOS Apple Silicon** (`aarch64-apple-darwin`): comes from Homebrew
(pre-installed on `macos-latest`, which is Apple Silicon). Every dependency
dylib is found via `otool -L` and relinked with `install_name_tool`:
`idevice_id`/`idevicebackup2` themselves (`externalBin` sidecars, placed in
`Contents/MacOS/` at runtime) reference dylibs via
`@executable_path/../Resources/<dylib>`, since the dylibs are `resources`
entries and land in the *different* `Contents/Resources/` directory;
dylib-to-dylib references use `@loader_path/<dylib>` since they all sit
together in that same flat directory. **CI-verified**: a `macos-latest`
runner reproduced the real `Contents/MacOS` + `Contents/Resources` split
and ran both binaries standalone successfully. Not run on an actual Mac by
a human, since none was available while building this.

**macOS Intel** (`x86_64-apple-darwin`): **not covered yet.** Homebrew on
an Apple Silicon runner doesn't natively cross-build for x86_64 (that would
need a separate Rosetta-based Homebrew prefix bootstrapped under
`/usr/local`, which wasn't attempted). `release.yml`'s Intel build ships
without bundled device tools until this is addressed - `check_prerequisites`
correctly reports them missing there, with PATH as a fallback for a
developer who has libimobiledevice installed locally.

**ffmpeg / ImageMagick**: not wired into the ingest or viewer pipeline at
all yet - media playback so far relies on formats the WebView's `<img>`/
`<video>` support natively.
