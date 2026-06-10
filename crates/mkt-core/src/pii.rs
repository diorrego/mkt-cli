//! Normalization and hashing of personally identifiable information (PII)
//! for audience uploads.
//!
//! Every ad platform (Meta, Google Customer Match, TikTok DMP, LinkedIn DMP)
//! requires user identifiers to be SHA-256 hashed after normalization.
//! The normalization rules implemented here follow the strictest common
//! contract so a single hasher works across all providers:
//!
//! - **Emails**: trim whitespace, lowercase.
//! - **Phones**: digits only (strip `+`, spaces, dashes, parentheses),
//!   drop leading zeros. Callers should pass numbers with country code.
//!
//! Values that already look like SHA-256 hex digests (64 hex chars) are
//! passed through unchanged so callers can mix pre-hashed and raw data.

use sha2::{Digest, Sha256};

/// Returns `true` if the value already looks like a SHA-256 hex digest.
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// SHA-256 hash a value, returning the lowercase hex digest.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

/// Normalize an email address per the cross-platform contract:
/// trim surrounding whitespace and lowercase.
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Normalize a phone number per the cross-platform contract:
/// keep digits only and strip leading zeros.
#[must_use]
pub fn normalize_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    let trimmed = digits.trim_start_matches('0');
    trimmed.to_string()
}

/// Normalize and hash an email address.
///
/// Already-hashed values (64 hex chars) are passed through unchanged.
#[must_use]
pub fn hash_email(email: &str) -> String {
    if is_sha256_hex(email) {
        return email.to_lowercase();
    }
    sha256_hex(&normalize_email(email))
}

/// Normalize and hash a phone number.
///
/// Already-hashed values (64 hex chars) are passed through unchanged.
#[must_use]
pub fn hash_phone(phone: &str) -> String {
    if is_sha256_hex(phone) {
        return phone.to_lowercase();
    }
    sha256_hex(&normalize_phone(phone))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-256 of "test@example.com" (well-known reference vector).
    const TEST_EMAIL_HASH: &str =
        "973dfe463ec85785f5f95af5ba3906eedb2d931c24e69824a89ea65dba4e813b";

    #[test]
    fn sha256_hex_known_vector() {
        assert_eq!(sha256_hex("test@example.com"), TEST_EMAIL_HASH);
    }

    #[test]
    fn normalize_email_trims_and_lowercases() {
        assert_eq!(
            normalize_email("  John.Doe@Example.COM  "),
            "john.doe@example.com"
        );
    }

    #[test]
    fn normalize_phone_strips_symbols_and_leading_zeros() {
        assert_eq!(normalize_phone("+1 (555) 123-4567"), "15551234567");
        assert_eq!(normalize_phone("0044 20 7946 0958"), "442079460958");
    }

    #[test]
    fn hash_email_normalizes_before_hashing() {
        // "  Test@Example.COM " normalizes to "test@example.com".
        assert_eq!(hash_email("  Test@Example.COM "), TEST_EMAIL_HASH);
    }

    #[test]
    fn hash_email_passes_through_existing_hash() {
        let upper = TEST_EMAIL_HASH.to_uppercase();
        assert_eq!(hash_email(TEST_EMAIL_HASH), TEST_EMAIL_HASH);
        assert_eq!(hash_email(&upper), TEST_EMAIL_HASH);
    }

    #[test]
    fn hash_phone_normalizes_before_hashing() {
        assert_eq!(hash_phone("+1 (555) 123-4567"), sha256_hex("15551234567"));
    }

    #[test]
    fn hash_phone_passes_through_existing_hash() {
        let hashed = sha256_hex("15551234567");
        assert_eq!(hash_phone(&hashed), hashed);
    }

    #[test]
    fn non_hash_64_char_value_is_hashed_not_passed_through() {
        // 64 chars but not hex — must be hashed, not passed through.
        let value = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert_ne!(hash_email(value), value);
    }
}
