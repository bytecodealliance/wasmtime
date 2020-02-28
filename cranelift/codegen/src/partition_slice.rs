//! Rearrange the elements in a slice according to a predicate.

use core::mem;

/// Rearrange the elements of the mutable slice `s` such that elements where `p(t)` is true precede
/// the elements where `p(t)` is false.
///
/// The order of elements is not preserved, unless the slice is already partitioned.
///
/// Returns the number of elements where `p(t)` is true.
pub fn partition_slice<T, F>(s: &mut [T], mut p: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    // The iterator works like a deque which we can pop from both ends.
    let mut i = s.iter_mut();

    // Number of elements for which the predicate is known to be true.
    let mut pos = 0;

    loop {
        // Find the first element for which the predicate fails.
        let head = loop {
            match i.next() {
                Some(head) => {
                    if !p(&head) {
                        break head;
                    }
                }
                None => return pos,
            }
            pos += 1;
        };

        // Find the last element for which the predicate succeeds.
        let tail = loop {
            match i.next_back() {
                Some(tail) => {
                    if p(&tail) {
                        break tail;
                    }
                }
                None => return pos,
            }
        };

        // Swap the two elements into the right order.
        mem::swap(head, tail);
        pos += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::partition_slice;
    use alloc::vec::Vec;

    fn check(x: &[u32], want: &[u32]) {
        assert_eq!(x.len(), want.len());
        let want_count = want.iter().cloned().filter(|&x| x % 10 == 0).count();
        let mut v = Vec::new();
        v.extend(x.iter().cloned());
        let count = partition_slice(&mut v[..], |&x| x % 10 == 0);
        assert_eq!(v, want);
        assert_eq!(count, want_count);
    }

    #[test]
    fn empty() {
        check(&[], &[]);
    }

    #[test]
    fn singles() {
        check(&[0], &[0]);
        check(&[1], &[1]);
        check(&[10], &[10]);
    }

    #[test]
    fn doubles() {
        check(&[0, 0], &[0, 0]);
        check(&[0, 5], &[0, 5]);
        check(&[5, 0], &[0, 5]);
        check(&[5, 4], &[5, 4]);
    }

    #[test]
    fn longer() {
        check(&[1, 2, 3], &[1, 2, 3]);
        check(&[1, 2, 10], &[10, 2, 1]); // Note: 2, 1 order not required.
        check(&[1, 10, 2], &[10, 1, 2]); // Note: 1, 2 order not required.
        check(&[1, 20, 10], &[10, 20, 1]); // Note: 10, 20 order not required.
        check(&[1, 20, 3, 10], &[10, 20, 3, 1]);
        check(&[20, 3, 10, 1], &[20, 10, 3, 1]);
    }
}
