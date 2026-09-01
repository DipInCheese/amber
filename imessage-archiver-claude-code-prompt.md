# Amber — Claude Code kickoff prompt

> Paste everything below into Claude Code to start. The mandate is to deliver a
> **complete, installable desktop application** for macOS and Windows — not just
> source. "Done" is defined in §"Definition of done". Build in the milestone order
> given; the only checkpoint is a short summary after M1, then continue through to
> shipped installers.

---

You are helping me build and ship an open-source desktop app called **Amber**.
Read this whole brief and the companion `SPEC.md`. Your job is to deliver the
**complete, installable application** described here, all the way to downloadable
installers for macOS and Windows. Confirm the stack and ask any blocking
questions first, then scaffold the repo and start Milestone 1.

## What we're building

**Amber** — a local-first desktop app for **macOS and Windows** that pulls a full
iMessage/SMS conversation (years of history, with photos and videos) off an
iPhone or a Mac, stores it in a durable open archive format, and lets you browse
it in an iMessage-style reader. Everything runs on-device. No cloud, no accounts,
no telemetry. The promise: *your memories, preserved intact, openable decades from
now, on hardware that's yours.*

## The experience

- **Plug-and-play ingest.** Plug an iPhone into the PC or Mac and Amber "just
  works": detect the device, guide the one-time trust prompt, create/refresh a
  local backup, import the conversation — no manual iTunes step.
- **iMessage-style viewer.** Scroll the thread with date separators, timestamps,
  left/right bubbles, inline photos, playable videos, emoji, and reaction /
  edited / reply indicators.
- **Photos-style timeline scrubber.** A draggable timeline with month/year ticks
  that maps scroll position to date, to jump across years instantly.
- **`.amber` archive format.** Amber's documented, durable native file (see
  `SPEC.md`), easy to import and export.
- **Installable & self-contained.** Ships as a normal installer per OS; bundles
  the helper tools it needs so the user installs nothing extra (one documented
  exception on Windows, below).

## Definition of done (the completion bar)

Amber is finished when all of this is true, verified on both macOS and Windows:

1. I can download an **installer from the project's GitHub Releases** for my OS,
   install it, and launch a normally-branded app (name, icon, version).
2. On first run, Amber **checks prerequisites** and clearly guides me through any
   that are missing (macOS Full Disk Access; on Windows, Apple's Mobile Device USB
   driver).
3. I can **plug in an iPhone**, approve the trust prompt, and Amber creates/refreshes
   a local backup and imports a chosen conversation into a `.amber` file — with a
   real progress indicator. (Alternatively I can point it at a Mac's live database
   or an existing iPhone backup.)
4. I can **browse the conversation** in the viewer with working photos, videos,
   emoji, reactions, edits and replies, and use the **timeline scrubber** to jump
   to any point across the whole history — smoothly, at 7-year scale.
5. I can **export and re-open** a `.amber` file, and optionally export a
   throwaway HTML view.
6. Everything works **fully offline**; nothing leaves the machine.

## Non-negotiable constraints (already decided — do not relitigate)

1. **Read-only at the source, always.** Never write to or mutate `chat.db`, a
   device, or any backup. Operate on copies in Amber's own working directory.
2. **Local-only.** No network calls involving user data, ever. No analytics.
3. **License: GPL-3.0-or-later** (required — linked libraries are GPL/LGPL).
4. **Durability over cleverness.** The archive stays recoverable with standard
   tools even if Amber disappears — no bespoke binary formats (see `SPEC.md`).

## Data sources (three, one pipeline)

- `Source::Device` — an iPhone over USB. Amber orchestrates a local backup, then
  reads it. **Primary, plug-and-play path.**
- `Source::Backup { path, password }` — an existing iTunes / Apple Devices backup.
- `Source::LiveMac` — `~/Library/Messages/chat.db` on macOS (needs Full Disk
  Access). Fastest when on a Mac.

All three resolve to "a Messages SQLite DB + attachments in a temp dir," which
feeds one parser → one writer that emits a `.amber` archive.

### Plug-and-play: how it works and its honest limits

iOS does not expose the Messages DB over USB — data only leaves the phone via a
**backup**. So `Source::Device` = detect → ensure trust → create/refresh backup →
parse. Design around:

- **Trust prompt:** first connect requires tapping "Trust This Computer" + passcode.
  Unavoidable; detect the untrusted state and instruct, don't bypass.
- **Encryption:** Messages + attachments are in *unencrypted* backups, so usually
  no password is needed. If the device already forces encrypted backups, prompt
  for that password and pass it to crabapple. Never silently change the device's
  encryption setting.
- **First backup is slow/large; later ones are incremental** — show real progress.
- **Keep phone unlocked and connected**; warn about required free disk space.
- **Windows driver:** USB access needs usbmuxd, provided by **Apple's Mobile
  Device support** (free "Apple Devices" app / iTunes). Amber **cannot bundle
  Apple's driver** — detect its absence and link the user to it.

## Tech stack (verify current versions/APIs against docs before coding)

- **Core:** Rust, as a **Cargo workspace** (see project layout). **GUI:** Tauri v2.
- **iMessage parsing:** `imessage-database = "4"` — decodes `attributedBody`,
  reactions, edits, group chats.
- **iOS backup read/decrypt:** `crabapple = "0.4.7"`.
- **Device backup over USB:** the **libimobiledevice** stack (`idevicebackup2`,
  usbmuxd/lockdownd). Start by **shelling out to bundled `idevicebackup2` /
  `idevice_id` sidecar binaries** (simplest, keeps the LGPL dependency
  arm's-length); `rusty_libimobiledevice` bindings are a later option, and the
  pure-Rust `idevice` crate is pre-1.0 — don't hang the core backup path on it.
- **SQLite:** `rusqlite = { version = "0.40", features = ["blob", "bundled"] }`.
- **`.amber` container:** a `zip` crate; `sha2` for content-addressed media;
  `serde`/`serde_json` for `manifest.json`.
- **Media conversion:** `ffmpeg` (video/audio) and ImageMagick (HEIC→JPEG),
  **bundled as sidecars**; prefer macOS built-in `sips`/`afconvert` when present.
- **Frontend:** lightweight web frontend (React or Svelte + Vite) with a
  **virtualized list** (TanStack Virtual / react-window) — mandatory at 7-year
  scale. Serve media via Tauri's asset protocol; never copy media into the web
  layer. Pick the framework, state why, and proceed.

> Licenses: `imessage-database` + `crabapple` are GPL-3.0-or-later; libimobiledevice
> is LGPL-2.1; all compatible with Amber's GPL-3.0-or-later. Match each library's
> real, current API — don't guess signatures.

## Project layout (Cargo workspace)

```
amber/
├── crates/
│   ├── amber-core/      # domain model + imessage-database parsing (UI-independent)
│   ├── amber-format/    # .amber reader/writer per SPEC.md
│   └── amber-ingest/    # Source resolver: device backup, ios backup, mac live
├── src-tauri/           # Tauri app: commands, wiring, bundler config
│   └── tauri.conf.json
├── src/                 # frontend: viewer (virtualized) + timeline scrubber
├── resources/
│   ├── bin/             # bundled sidecars (idevicebackup2, ffmpeg, …) per-target
│   └── icons/           # app icon set
├── fixtures/            # tiny SYNTHETIC test DBs — never real data
├── .github/workflows/   # ci.yml (test), release.yml (build+publish installers)
├── SPEC.md  README.md  LICENSE  CONTRIBUTING.md  SECURITY.md  CODE_OF_CONDUCT.md
```

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

Tauri commands: `list_devices`, `check_prerequisites`, `ingest_source`
(→ `.amber`), `open_archive`, `list_conversations`, `query_messages` (windowed),
`get_day_index`, `export_view`. Keep the Rust core UI-independent and unit-tested.

## Milestones (build in order)

- **M1 — parsing slice (macOS live DB).** Given a `chat.db` + a contact, return
  JSON messages (text, timestamp, sender, `is_from_me`, attachment paths) for one
  conversation. **Prove `attributedBody` decodes** where `text` is empty.
  Unit-test the Apple-epoch conversion (`unix_ms = date/1e6 + 978307200000`,
  handling nanosecond vs legacy-second) against a synthetic fixture. Stub other
  sources/writer behind the seams. **Summarize the slice in your reply, then
  continue** to M2 (don't wait for me unless something is genuinely blocking).
- **M2 — `.amber` writer + reader** per `SPEC.md`: write a valid bundle (SQLite +
  `manifest.json` + sidecar attachments), reopen, round-trip + integrity tests.
- **M3 — viewer:** virtualized iMessage-style thread from a `.amber` — bubbles,
  date separators, timestamps, emoji, inline photos, playable video via the asset
  protocol, lazy media, reactions/edits/replies.
- **M4 — timeline scrubber:** draggable date timeline from `day_index`, month/year
  ticks, jump-to-date, synced with the virtualized list.
- **M5 — plug-and-play device ingest:** detect iPhone, guide trust, create/refresh
  backup via bundled `idevicebackup2`, decrypt via crabapple if needed, parse →
  `.amber`; incremental refresh; progress + prerequisite checks; first-run
  onboarding (Full Disk Access on macOS, Apple driver detection on Windows).
- **M6 — package & ship the installable app (see next section).**

## Packaging, installers & release (M6 — required for "done")

- **Installers via the Tauri bundler:** macOS `.app` + `.dmg` (Apple Silicon **and**
  Intel); Windows `.msi` (WiX) and/or `.exe` (NSIS). Set app identity: product
  name "Amber", a bundle identifier (use a placeholder like `app.amber.desktop`
  and tell me to confirm it), version, and a generated **app icon set** (an
  amber/resin-drop motif is fine as a first pass).
- **Bundle the helper binaries as Tauri `externalBin` sidecars** (`idevicebackup2`
  and its libimobiledevice deps, `ffmpeg`, ImageMagick) for each target, so the
  installed app is self-contained. The **one exception is Apple's Windows USB
  driver**, which can't be redistributed — detect and guide instead.
- **First-run onboarding / prerequisite screen:** verify bundled tools load,
  request macOS Full Disk Access with a walkthrough, and on Windows detect Apple's
  Mobile Device support and link to it if missing.
- **Code signing / notarization:** wire the config but treat certs as optional
  secrets (they cost money). Ship **unsigned installers** by default with a
  documented "open anyway" path (macOS Gatekeeper right-click-open; Windows
  SmartScreen → More info → Run anyway). Leave signing identity / cert thumbprint
  as CI secrets so it turns on later without code changes.
- **Release CI (`.github/workflows/release.yml`):** on a version tag, build the
  installers for macOS (both arches) and Windows and attach them to a **GitHub
  Release**. `ci.yml` runs tests + `cargo fmt --check` + `cargo clippy -D warnings`
  on `macos-latest` and `windows-latest` for every push/PR.
- **Auto-update (optional, later):** the Tauri updater against GitHub Releases;
  needs a signing key — stub it, don't block shipping on it.

## Open-source setup (start in M1, complete by M6)

- **LICENSE** GPL-3.0-or-later + SPDX headers. **README** (what it does, privacy
  promise, per-OS sources, install + prerequisites, credits to `imessage-database`,
  `crabapple`, libimobiledevice). **SPEC.md** kept in sync. **CONTRIBUTING**,
  **CODE_OF_CONDUCT**, **SECURITY**, issue/PR templates. **.gitignore** for Rust +
  Node + Tauri. **Never commit real data** (`chat.db`, backups, `.amber`) —
  synthetic fixtures only.

## Working agreement

- **Deliver the whole app through M6.** Don't stop and wait except at the M1
  summary or when genuinely blocked.
- **Never make a network call involving user data.** This overrides convenience.
- You may add well-scoped dependencies as needed; call out anything unusual, and
  anything that would make network requests.
- Commit in small, reviewable steps. Write tests alongside code. Keep the Rust
  core testable independent of the UI. Verify library APIs against current docs
  and flag version drift. Flag any assumption you had to make.

**Start by confirming the stack, raising any blocking questions, then scaffolding
the workspace and Milestone 1.**
