# Contributing to Amber

Thanks for your interest in Amber. It's early — the project is still working
through its first milestones — so the best contributions right now are:

- Bug reports with a clear repro (see [SECURITY.md](SECURITY.md) instead if
  it's a security issue).
- Small, focused pull requests against an open issue.
- Feedback on [`SPEC.md`](SPEC.md), the `.amber` archive format.

## Ground rules

- **Never commit real data.** No real `chat.db`, device backups, or `.amber`
  files — ever, including in test fixtures. Use tiny synthetic fixtures
  (`fixtures/`) instead. `.gitignore` blocks the obvious filenames, but that's
  a backstop, not a substitute for checking `git status` before you commit.
- **Read-only at the source.** Code that touches a device, a backup, or
  `chat.db` must never write to or mutate them. Operate on copies in Amber's
  own working directory.
- **Local-only.** No network calls involving user data. Flag any new
  dependency that makes network requests.
- Keep the Rust core (`amber-core`, `amber-format`, `amber-ingest`)
  UI-independent and unit-tested.

## Development setup

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs tests, `cargo fmt --check`, and `cargo clippy -D warnings` on both
`macos-latest` and `windows-latest` for every push and PR — please run the
same locally before opening one.

## Commit style

Small, reviewable commits. Write tests alongside code changes. If you had to
make an assumption about an external library's API or a platform behavior,
say so in the PR description.

## License

By contributing, you agree your contributions are licensed under
[GPL-3.0-or-later](LICENSE), the same as the rest of the project.
