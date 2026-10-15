# Security Policy

Amber handles sensitive personal data by design: years of private messages,
photos, and videos, plus (during ingest) a full iOS device backup. Security
issues are taken seriously.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Instead, use GitHub's [private vulnerability reporting](../../security/advisories/new)
for this repository. If that isn't available, open a regular issue asking a
maintainer to reach out privately, without describing the vulnerability
itself.

Please include:

- A description of the issue and its potential impact.
- Steps to reproduce, or a proof of concept.
- The affected version/commit.

We'll acknowledge reports as promptly as we can and keep you updated as the
issue is triaged and fixed.

## Scope

Amber's core security properties, in priority order:

1. **Read-only at the source.** Amber must never write to or mutate a
   device, an existing backup, or `chat.db`.
2. **Local-only.** Amber must never make a network call involving user data.
   A vulnerability that causes user data (messages, media, backup contents)
   to leave the device is critical.
3. **The `.amber` archive is plaintext.** This is a documented, deliberate
   tradeoff (see `SPEC.md` §6) in favor of durability with standard tools —
   not a vulnerability report on its own. Treat `.amber` files like any other
   sensitive personal file.

4. **The saved phone backup is plaintext on disk** (unless the phone's own
   backup encryption is on). iOS can't back up less than the whole device,
   so Amber keeps the result in its app-data folder
   (`device-backups/` under the app's local data directory) so that later
   imports are incremental and other conversations can be picked without the
   phone. This is deliberate, not a vulnerability report on its own, but it
   is a full copy of the phone's data: the import window shows its size and
   has a **Delete saved backup** button. Anything that makes Amber leave one
   behind after that button is pressed, or read outside that folder, is in
   scope.

Vulnerabilities in bundled third-party sidecars (`idevicebackup2`, `ffmpeg`,
ImageMagick) should generally be reported upstream, but let us know too if
Amber's use of them is what makes an issue exploitable.
