// SPDX-License-Identifier: GPL-3.0-or-later
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

fn to_hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Lowercase hex SHA-256 digest of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(&Sha256::digest(bytes))
}

/// Lowercase hex SHA-256 digest and size in bytes of the file at `path`,
/// read in chunks so multi-gigabyte media never has to fit in memory.
pub fn sha256_file(path: &Path) -> std::io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok((to_hex(&hasher.finalize()), size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_file_hash_matches_the_in_memory_one() {
        // Bigger than the 1 MiB read buffer, so it takes several chunks.
        let bytes: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), &bytes).unwrap();

        let (hash, size) = sha256_file(file.path()).unwrap();

        assert_eq!(hash, sha256_hex(&bytes));
        assert_eq!(size, bytes.len() as u64);
    }

    #[test]
    fn matches_known_digest() {
        // echo -n "" | sha256sum
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
