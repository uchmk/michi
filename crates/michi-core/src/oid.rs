//! An object's name: its hash, 20 bytes (SHA-1) or 32 (SHA-256), kept as
//! bytes so a hundred thousand of them stay small and quick to compare.

use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid {
    bytes: [u8; 32],
    len: u8,
}

impl Oid {
    /// From 40 or 64 hex digits, either case.
    pub fn parse(hex: &str) -> Option<Oid> {
        let hex = hex.as_bytes();
        if hex.len() != 40 && hex.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (i, pair) in hex.chunks(2).enumerate() {
            bytes[i] = (digit(pair[0])? << 4) | digit(pair[1])?;
        }
        Some(Oid { bytes, len: (hex.len() / 2) as u8 })
    }

    /// All zeros: what git prints for a branch with no commit yet.
    pub fn is_zero(&self) -> bool {
        self.bytes.iter().all(|&b| b == 0)
    }

    /// The first 7 hex digits, as the graph's last column shows it.
    pub fn short(&self) -> String {
        let mut s = self.to_string();
        s.truncate(7);
        s
    }

    /// Whether the hex starts with `prefix` (lower case).
    pub fn starts_with(&self, prefix: &str) -> bool {
        prefix.len() <= self.len as usize * 2 && self.to_string().starts_with(prefix)
    }
}

fn digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.bytes[..self.len as usize] {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({})", self.short())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_round_trip() {
        let sha1 = "0123456789abcdef0123456789ABCDEF01234567";
        let id = Oid::parse(sha1).unwrap();
        assert_eq!(id.to_string(), sha1.to_lowercase());
        assert_eq!(id.short(), "0123456");
        assert!(id.starts_with("01234") && !id.starts_with("1"));
        let sha256 = "ab".repeat(32);
        assert_eq!(Oid::parse(&sha256).unwrap().to_string(), sha256);
        assert_ne!(Oid::parse(&"ab".repeat(20)), Oid::parse(&format!("{}{}", "ab".repeat(20), "00".repeat(12))));
    }

    #[test]
    fn what_is_not_a_hash_is_refused() {
        assert_eq!(Oid::parse(""), None);
        assert_eq!(Oid::parse("0123456"), None);
        assert_eq!(Oid::parse(&"g".repeat(40)), None);
        assert!(Oid::parse(&"0".repeat(40)).unwrap().is_zero());
    }
}
