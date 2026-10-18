//! Shared encoding/hashing for high-entropy opaque credentials.

use sha2::{Digest, Sha256};

pub(crate) fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub(crate) fn encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}")
            .expect("formatting into a String cannot fail");
        output
    })
}
