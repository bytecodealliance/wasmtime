//! Build support for precomputed constant hash tables.
//!
//! This module can generate constant hash tables using open addressing and quadratic probing.
//!
//! The hash tables are arrays that are guaranteed to:
//!
//! - Have a power-of-two size.
//! - Contain at least one empty slot.
//!
//! This module provides build meta support for lookups in these tables, as well as the shared hash
//! function used for probing.

use std::iter;

/// A primitive hash function for matching opcodes.
pub fn simple_hash(s: &str) -> usize {
    let mut h: u32 = 5381;
    for c in s.chars() {
        h = (h ^ c as u32).wrapping_add(h.rotate_right(6));
    }
    h as usize
}

/// Compute an open addressed, quadratically probed hash table containing
/// `items`. The returned table is a list containing the elements of the
/// iterable `items` and `None` in unused slots.
#[allow(clippy::float_arithmetic)]
pub fn generate_table<'cont, T, I: iter::Iterator<Item = &'cont T>, H: Fn(&T) -> usize>(
    items: I,
    num_items: usize,
    hash_function: H,
) -> Vec<Option<&'cont T>> {
    let size = (1.20 * num_items as f64) as usize;

    // Probing code's stop condition relies on the table having one vacant entry at least.
    let size = if size.is_power_of_two() {
        size * 2
    } else {
        size.next_power_of_two()
    };

    let mut table = vec![None; size];

    for i in items {
        let mut h = hash_function(&i) % size;
        let mut s = 0;
        while table[h].is_some() {
            s += 1;
            h = (h + s) % size;
        }
        table[h] = Some(i);
    }

    table
}

#[cfg(test)]
mod tests {
    use super::{generate_table, simple_hash};

    #[test]
    fn basic() {
        assert_eq!(simple_hash("Hello"), 0x2fa70c01);
        assert_eq!(simple_hash("world"), 0x5b0c31d5);
    }

    #[test]
    fn test_generate_table() {
        let v = vec!["Hello".to_string(), "world".to_string()];
        let table = generate_table(v.iter(), v.len(), |s| simple_hash(&s));
        assert_eq!(
            table,
            vec![
                None,
                Some(&"Hello".to_string()),
                Some(&"world".to_string()),
                None
            ]
        );
    }
}
