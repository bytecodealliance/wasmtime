/// A primitive hash function for matching opcodes.
/// Must match `meta/constant_hash.py`.
pub fn simple_hash(s: &str) -> u32 {
    let mut h: u32 = 5381;
    for c in s.chars() {
        h = (h ^ c as u32).wrapping_add(h.rotate_right(6));
    }
    h
}

#[cfg(test)]
mod tests {
    use super::simple_hash;

    #[test]
    fn basic() {
        // c.f. meta/constant_hash.py tests.
        assert_eq!(simple_hash("Hello"), 0x2fa70c01);
        assert_eq!(simple_hash("world"), 0x5b0c31d5);
    }
}
