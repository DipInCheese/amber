# Amber

A local-first desktop app for **macOS and Windows** that pulls a full
iMessage/SMS conversation — years of history, with photos and videos — off an
iPhone or a Mac, stores it in a durable open archive format, and lets you
browse it in an iMessage-style reader.

Everything runs on-device. No cloud, no accounts, no telemetry.

> Your memories, preserved intact, openable decades from now, on hardware
> that's yours.

## Status

All six milestones have landed (see [Roadmap](#roadmap)) - parsing,
the `.amber` format, the viewer, the timeline scrubber, plug-and-play
ingest, and packaging. It's not fully battle-tested yet: the device/live-Mac
ingest paths haven't been exercised against real hardware in the environment
that built them, sidecar tools (`idevicebackup2` et al.) aren't bundled into
installers yet, and imported conversations don't carry edits/replies/
reactions until amber-core's parser is extended to extract them. Treat it as
a working first release candidate, not a finished product.

## The experience

- **Plug-and-play ingest.** Plug an iPhone into the PC or Mac and Amber
  detects the device, guides the one-time trust prompt, creates/refreshes a
  local backup, and imports the conversation — no manual iTunes step.
- **iMessage-style viewer.** Scroll the thread with date separators,
  timestamps, left/right bubbles, inline photos, playable videos, emoji, and
  reaction / edited / reply indicators.
- **Photos-style timeline scrubber.** A draggable timeline with month/year
  ticks that maps scroll position to date, to jump across years instantly.
- **The `.amber` archive format.** Amber's documented, durable native file
  (see [`SPEC.md`](SPEC.md)) — a ZIP wrapping a SQLite database and
  content-addressed media, readable with standard tools even if Amber is
  never updated again.

## Privacy

- **Local-only.** No network calls involving user data, ever. No analytics,
  no accounts.
- **Read-only at the source.** Amber never writes to or mutates `chat.db`, a
  device, or any backup — it always operates on copies in its own working
  directory.

## Data sources

Three sources feed one pipeline (device backup → parse → `.amber`):

| Source | What it needs |
| --- | --- |
| **iPhone over USB** (primary, plug-and-play) | Trust the computer on the phone; on Windows, [Apple Mobile Device support](https://support.apple.com/HT204074) (via iTunes or the Apple Devices app) for the USB driver |
| **An existing iTunes / Apple Devices backup** | The backup's password, if it's encrypted |
| **A Mac's live Messages database** | Full Disk Access for Amber |

## Install

Once a maintainer pushes a `v*.*.*` tag, [`release.yml`](.github/workflows/release.yml)
builds installers for macOS (Apple Silicon + Intel) and Windows and attaches
them as a **draft** [GitHub Release](../../releases) for review before
publishing. Installers are unsigned by default (no certificates are
configured yet - see [Code signing](#code-signing) below):

- **macOS:** right-click the `.app` → Open → Open, to bypass Gatekeeper's
  "unidentified developer" warning (only needed the first time).
- **Windows:** SmartScreen will flag the `.msi`/`.exe` → click "More info" →
  "Run anyway".

### Code signing

Unset by default; `release.yml` picks up certificates automatically once
these repository secrets exist, no workflow changes needed:

| Platform | Secrets |
| --- | --- |
| macOS (sign + notarize) | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `KEYCHAIN_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| Windows | `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` |

## Building from source

Requires a recent stable [Rust toolchain](https://rustup.rs) and
[Node.js](https://nodejs.org).

```bash
npm install
cargo build --workspace
cargo test --workspace
npx tsc --noEmit
npm run tauri dev   # launches the desktop app
```

**Frontend stack:** React + TypeScript + Vite, with
[TanStack Virtual](https://tanstack.com/virtual) for the message list
(mandatory at 7-year/tens-of-thousands-of-messages scale) — chosen for
Tauri's first-class React support and TanStack Virtual's maturity over the
Svelte alternative. Attachments are served straight out of the open
`.amber`'s ZIP by a custom `amber-attachment://` Tauri protocol
([`src-tauri/src/protocol.rs`](src-tauri/src/protocol.rs)), never copied into
the web layer.

## Roadmap

Building in order; each milestone lands as its own set of commits.

- [x] **M1 — parsing slice.** Given a `chat.db` and a chat identifier,
      produce `DomainMessage`s (text, timestamp, sender, attachments),
      decoding `attributedBody` where `text` is empty.
- [x] **M2 — `.amber` writer + reader** per `SPEC.md`: writes a valid
      bundle, reopens it, and round-trips every table plus content-addressed
      attachments and the integrity check.
- [x] **M3 — viewer:** virtualized iMessage-style thread from a `.amber` -
      bubbles, date separators, timestamps, emoji, inline photos/video,
      reactions, edited/unsent/reply indicators.
- [x] **M4 — timeline scrubber.** Draggable, Photos-style vertical timeline
      with month/year ticks; position maps linearly to calendar date, jumps
      the virtualized thread instantly, and stays in sync as you scroll.
- [x] **M5 — plug-and-play device ingest.** `amber-ingest` resolves an
      iPhone over USB, an existing backup, or a live Mac to a Messages
      database; `check_prerequisites`/`list_devices`/`begin_ingest`/
      `ingest_conversation` wire it into the app with a real progress
      indicator and an in-app Import panel. The device/live-Mac paths are
      written against verified APIs (`idevicebackup2`'s own `--help`/source,
      `crabapple`'s real crate API) but untested against actual hardware in
      this environment - see the caveats in `crates/amber-ingest/src/device.rs`
      and `live_mac.rs`. Known gap: imports currently carry text/timestamp/
      sender/attachments only (no edits, replies, or reactions yet) - tracked
      separately.
- [x] **M6 — package & ship installers** for macOS and Windows.
      `release.yml` builds and attaches installers to a draft GitHub Release
      on a version tag. Verified locally on Windows: `npm run tauri build`
      produces a real, working `.msi` and `.exe` (NSIS) - both launch the
      actual app. macOS build/signing is configured but unverified here (no
      Mac available in this environment; CI will be the first real test).
      Known gap: the `idevicebackup2`/`idevice_id`/ffmpeg/ImageMagick
      sidecars aren't bundled yet (`externalBin`) - see
      [`resources/bin/README.md`](resources/bin/README.md); until then,
      device ingest works only when those tools are on `PATH`.

## Architecture

```
Source ── Resolver ──> Messages DB + attachments (temp)
   │                          │
   ▼                          ▼
Device: idevicebackup2 →    Reader (amber-core)  ──> domain structs
        backup → crabapple           │
Backup: crabapple → extract          ▼
LiveMac: copy chat.db        Writer (amber-format) ──> .amber bundle
                                       │
                                       ▼
                        Viewer (Tauri webview): virtualized thread +
                        timeline scrubber, reading from the .amber file
```

| Crate | Responsibility |
| --- | --- |
| [`amber-core`](crates/amber-core) | Domain model + iMessage/SMS parsing. UI-independent. |
| [`amber-format`](crates/amber-format) | `.amber` reader/writer per [`SPEC.md`](SPEC.md). |
| [`amber-ingest`](crates/amber-ingest) | Resolves a device/backup/live-Mac source to a database. |
| [`src-tauri`](src-tauri) | The desktop app shell: Tauri commands, the `amber-attachment` protocol, window/bundle config. |
| [`src`](src) | The React viewer: virtualized thread, bubbles, timeline scrubber. |

## Credits

Amber is built on the work of:

- [`imessage-database`](https://crates.io/crates/imessage-database) —
  parses iMessage/SMS `chat.db` databases, including `attributedBody`
  decoding, reactions, edits, and group chats.
- [`crabapple`](https://crates.io/crates/crabapple) — reads and decrypts iOS
  device backups.
- [libimobiledevice](https://libimobiledevice.org/) — talks to iOS devices
  over USB (device backup creation/refresh).

## License

[GPL-3.0-or-later](LICENSE).
