# Amber

A local-first desktop app for **macOS and Windows** that pulls a full
iMessage/SMS conversation — years of history, with photos and videos — off an
iPhone or a Mac, stores it in a durable open archive format, and lets you
browse it in an iMessage-style reader.

Everything runs on-device. No cloud, no accounts, no telemetry.

> Your memories, preserved intact, openable decades from now, on hardware
> that's yours.

## Status

Amber is under active development, building milestone by milestone (see
[Roadmap](#roadmap) below). It is not yet ready for daily use.

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

Amber isn't shipping installers yet — see the [Roadmap](#roadmap). Once M6
lands, signed-or-not installers for macOS (`.dmg`) and Windows (`.msi`/`.exe`)
will be attached to [GitHub Releases](../../releases).

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
- [ ] **M5 — plug-and-play device ingest.**
- [ ] **M6 — package & ship installers** for macOS and Windows.

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
