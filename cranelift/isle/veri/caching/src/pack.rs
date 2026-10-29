//! Binary "pack" format for batches of cache entries.
//!
//! A pack is a single object in the backing store holding many cache entries,
//! sorted by key. Both base packs and delta packs use this format; a base
//! additionally records the names of the delta packs whose entries it already
//! includes (its "covered" deltas), so that readers know which deltas still
//! need to be applied on top of it.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! magic           8 bytes   b"IVCPACK1"
//! covered count   u32
//!   name len      u16       } repeated `covered count` times
//!   name          utf-8     }
//! entry count     u64
//!   key           32 bytes  } repeated `entry count` times, sorted by key
//!   value len     u32       }
//!   value         utf-8     } JSON-encoded solver response
//! ```

use std::collections::BTreeMap;
use std::io;

/// A cache key: the SHA-256 digest of the solver name and replay script.
pub(crate) type Key = [u8; 32];

const MAGIC: &[u8; 8] = b"IVCPACK1";

/// The decoded contents of a pack.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Pack {
    /// Names of delta packs whose entries are included in this pack. Empty
    /// for delta packs.
    pub covered: Vec<String>,
    /// Entries, keyed by cache key; values are JSON-encoded responses.
    pub entries: BTreeMap<Key, String>,
}

impl Pack {
    /// Serialize this pack.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.entries.len() * 48);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&u32::try_from(self.covered.len()).unwrap().to_le_bytes());
        for name in &self.covered {
            out.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        out.extend_from_slice(&u64::try_from(self.entries.len()).unwrap().to_le_bytes());
        for (key, value) in &self.entries {
            out.extend_from_slice(key);
            out.extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
            out.extend_from_slice(value.as_bytes());
        }
        out
    }

    /// Deserialize a pack, rejecting truncated or malformed input.
    pub fn decode(mut bytes: &[u8]) -> io::Result<Pack> {
        fn take<'a>(bytes: &mut &'a [u8], n: usize) -> io::Result<&'a [u8]> {
            if bytes.len() < n {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "truncated cache pack",
                ));
            }
            let (head, tail) = bytes.split_at(n);
            *bytes = tail;
            Ok(head)
        }
        fn utf8(b: &[u8]) -> io::Result<String> {
            String::from_utf8(b.to_vec()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }

        if take(&mut bytes, MAGIC.len())? != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bad cache pack magic",
            ));
        }
        let covered_count = u32::from_le_bytes(take(&mut bytes, 4)?.try_into().unwrap());
        let mut covered = Vec::new();
        for _ in 0..covered_count {
            let len = u16::from_le_bytes(take(&mut bytes, 2)?.try_into().unwrap());
            covered.push(utf8(take(&mut bytes, usize::from(len))?)?);
        }
        let entry_count = u64::from_le_bytes(take(&mut bytes, 8)?.try_into().unwrap());
        let mut entries = BTreeMap::new();
        for _ in 0..entry_count {
            let key: Key = take(&mut bytes, 32)?.try_into().unwrap();
            let len = u32::from_le_bytes(take(&mut bytes, 4)?.try_into().unwrap());
            let value = utf8(take(&mut bytes, len as usize)?)?;
            entries.insert(key, value);
        }
        if !bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "trailing bytes in cache pack",
            ));
        }
        Ok(Pack { covered, entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut pack = Pack::default();
        pack.covered.push("delta/0001-abcd.pack".to_string());
        pack.entries.insert([1; 32], "\"sat\"".to_string());
        pack.entries.insert([0; 32], "[\"x\",\"y\"]".to_string());
        let bytes = pack.encode();
        assert_eq!(Pack::decode(&bytes).unwrap(), pack);
        assert!(Pack::decode(&bytes[..bytes.len() - 1]).is_err());
        assert!(Pack::decode(b"nope").is_err());
    }
}
