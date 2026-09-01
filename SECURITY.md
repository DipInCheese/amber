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

Vulnerabilities in bundled third-party sidecars (`idevicebackup2`, `ffmpeg`,
ImageMagick) should generally be reported upstream, but let us know too if
Amber's use of them is what makes an issue exploitable.
