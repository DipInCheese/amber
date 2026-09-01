# Bundled sidecar binaries

This directory is where per-target `idevicebackup2`, `idevice_id`, `ffmpeg`,
and ImageMagick binaries belong once Amber bundles them as Tauri
[`externalBin`](https://v2.tauri.app/develop/sidecar/) sidecars, so an
installed app needs nothing extra on the user's machine.

**Not populated yet.** Producing real, per-target (macOS arm64/x86_64,
Windows x64) libimobiledevice/ffmpeg/ImageMagick binaries — building or
vendoring them, verifying licenses, wiring the `externalBin` paths into
`src-tauri/tauri.conf.json` — is real infrastructure work this environment
couldn't do (no macOS/cross-compilation toolchain available here, and these
aren't things to fetch from an arbitrary URL without verifying provenance).

Until this is done:

- `amber-ingest::check_prerequisites` correctly reports these tools as
  missing and tells the user why (see `crates/amber-ingest/src/prerequisites.rs`).
- `amber-ingest::device` falls back to resolving `idevice_id`/`idevicebackup2`
  from `PATH` (see `src-tauri/src/tools.rs`), so a developer with
  libimobiledevice installed locally can still exercise the device-ingest
  flow.
- `ffmpeg`/ImageMagick aren't wired into the ingest or viewer pipeline at
  all yet - media playback so far relies on formats the WebView's `<img>`/
  `<video>` support natively.

**To finish this:** place the real binaries here, named
`<tool>-<rust-target-triple>[.exe]` per Tauri's sidecar convention (e.g.
`idevicebackup2-x86_64-pc-windows-msvc.exe`,
`idevicebackup2-aarch64-apple-darwin`), add `"externalBin": ["../resources/bin/idevicebackup2", "../resources/bin/idevice_id"]`
(and ffmpeg/ImageMagick once wired up) to `src-tauri/tauri.conf.json`'s
`bundle` config, and update `src-tauri/src/tools.rs` to resolve them via
Tauri's `resource_dir()`/sidecar path instead of the current bundled-dir
existence check.
