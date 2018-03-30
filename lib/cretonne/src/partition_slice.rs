//! Rearrange the elements in a slice according to a predicate.

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
    // Count the length of the prefix where `p` returns true.
    let mut count = match s.iter().position(|t| !p(t)) {
        Some(t) => t,
        None => return s.len(),
    };

    // Swap remaining `true` elements into place.
    //
    // This actually preserves the order of the `true` elements, but the `false` elements get
    // shuffled.
    for i in count + 1..s.len() {
        if p(&s[i]) {
            s.swap(count, i);
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::partition_slice;
    use std::vec::Vec;

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
        check(&[1, 20, 10], &[20, 10, 1]);
        check(&[1, 20, 3, 10], &[20, 10, 3, 1]);
        check(&[20, 3, 10, 1], &[20, 10, 3, 1]);
    }
}
