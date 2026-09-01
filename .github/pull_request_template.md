## What does this change?

## Why?

## Checklist

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all pass locally
- [ ] No real `chat.db`, device backup, or `.amber` file is included anywhere in this diff — synthetic fixtures only
- [ ] Any code that touches a device, backup, or `chat.db` is read-only at the source
- [ ] Any new dependency that makes network requests is called out below

## Notes for reviewers
