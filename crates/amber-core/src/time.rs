// SPDX-License-Identifier: GPL-3.0-or-later
//! Conversion between Apple's Messages timestamp format and Unix milliseconds.
//!
//! Apple stores message dates as an offset from `2001-01-01T00:00:00Z`. Modern
//! databases (macOS Big Sur+ / iOS 11+) store nanoseconds; older databases store
//! whole seconds. There's no explicit flag for which one a given database uses,
//! so we detect it the same way `imessage-database` does: a nanosecond value is
//! many orders of magnitude larger than a seconds value for any realistic date,
//! so a magnitude threshold well above legacy timestamps but well below modern
//! nanosecond ones disambiguates the two.

/// Milliseconds between the Unix epoch (1970-01-01) and the Apple Messages
/// epoch (2001-01-01), i.e. `978_307_200 * 1000`.
pub const APPLE_EPOCH_UNIX_MS: i64 = 978_307_200_000;

/// Raw values at or above this magnitude are treated as nanoseconds; below it,
/// as whole seconds. Legacy (pre-2011) timestamps in seconds since 2001 top out
/// around 10^9; modern nanosecond timestamps for any realistic date exceed 10^17.
/// `10^12` sits comfortably between the two.
const NANOSECOND_MAGNITUDE_THRESHOLD: i64 = 1_000_000_000_000;

/// Convert a raw Apple Messages timestamp (nanoseconds or whole seconds since
/// `2001-01-01T00:00:00Z`, per the detection above) to Unix milliseconds (UTC).
///
/// `0` is Apple's own "unset" sentinel for optional date columns (`date_read`,
/// `date_delivered`, `date_edited`); it round-trips to `0` here rather than to
/// the Messages epoch, so callers can keep testing those fields with `!= 0`.
pub fn apple_ts_to_unix_ms(raw: i64) -> i64 {
    if raw == 0 {
        return 0;
    }
    if raw.abs() >= NANOSECOND_MAGNITUDE_THRESHOLD {
        raw.div_euclid(1_000_000) + APPLE_EPOCH_UNIX_MS
    } else {
        raw * 1000 + APPLE_EPOCH_UNIX_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2023-01-01T00:00:00Z, computed independently of the function under test.
    const KNOWN_UNIX_MS: i64 = 1_672_531_200_000;

    #[test]
    fn converts_modern_nanosecond_timestamps() {
        let raw_ns = (KNOWN_UNIX_MS - APPLE_EPOCH_UNIX_MS) * 1_000_000;
        assert_eq!(apple_ts_to_unix_ms(raw_ns), KNOWN_UNIX_MS);
    }

    #[test]
    fn converts_legacy_second_timestamps() {
        let raw_secs = (KNOWN_UNIX_MS - APPLE_EPOCH_UNIX_MS) / 1000;
        assert_eq!(apple_ts_to_unix_ms(raw_secs), KNOWN_UNIX_MS);
    }

    #[test]
    fn zero_is_treated_as_unset_not_the_epoch() {
        assert_eq!(apple_ts_to_unix_ms(0), 0);
    }

    #[test]
    fn one_second_after_the_apple_epoch_as_legacy_seconds() {
        assert_eq!(apple_ts_to_unix_ms(1), APPLE_EPOCH_UNIX_MS + 1000);
    }

    #[test]
    fn threshold_boundary_is_treated_as_nanoseconds() {
        // Exactly at NANOSECOND_MAGNITUDE_THRESHOLD: 1000 seconds after the
        // Apple epoch, expressed in nanoseconds.
        assert_eq!(
            apple_ts_to_unix_ms(1_000_000_000_000),
            APPLE_EPOCH_UNIX_MS + 1_000_000
        );
    }
}
