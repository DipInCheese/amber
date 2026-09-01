# `AttributedBodyTextOnly.bin`

A real macOS Messages `attributedBody` (NSKeyedArchiver `typedstream`) BLOB that
decodes to the plain text `"Noter test"`. Used by `amber-core`'s parser tests
to prove attributedBody decoding without needing a real device or Mac.

Sourced from the `imessage-database` crate's own test fixtures
(`imessage-database/test_data/typedstream/AttributedBodyTextOnly`,
https://github.com/ReagentX/imessage-exporter), which is GPL-3.0-or-later —
license-compatible with Amber. Not real user data.
