use crate::Output;
use std::cmp;
use std::hash::Hash;

impl Output for u64 {
    fn empty() -> Self {
        0
    }

    fn prefix(a: &Self, b: &Self) -> Self {
        cmp::min(*a, *b)
    }

    fn difference(a: &Self, b: &Self) -> Self {
        a - b
    }

    fn concat(a: &Self, b: &Self) -> Self {
        a + b
    }
}

impl<T> Output for Vec<T>
where
    T: Clone + Eq + Hash,
{
    fn empty() -> Self {
        vec![]
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn prefix(a: &Self, b: &Self) -> Self {
        a.iter()
            .cloned()
            .zip(b.iter().cloned())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect()
    }

    fn difference(a: &Self, b: &Self) -> Self {
        let i = a
            .iter()
            .zip(b.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cmp::min(a.len(), b.len()));
        a[i..].to_vec()
    }

    fn concat(a: &Self, b: &Self) -> Self {
        let mut c = a.clone();
        c.extend(b.iter().cloned());
        c
    }
}

#[cfg(test)]
mod tests {
    use crate::Output;
    use std::fmt::Debug;

    // Assert the laws that `Output` requires for correctness. `a` and `b`
    // should be two different instances of an `Output` type.
    fn assert_laws<O>(a: O, b: O)
    where
        O: Clone + Debug + Output,
    {
        // Law 1
        assert_eq!(O::concat(&O::empty(), &a), a.clone());

        // Law 2
        assert_eq!(O::prefix(&b, &a), O::prefix(&a, &b));

        // Law 3
        assert_eq!(O::prefix(&O::empty(), &a), O::empty());

        // Law 4
        assert_eq!(O::difference(&O::concat(&a, &b), &a), b);
    }

    #[test]
    fn impl_for_u64() {
        assert_laws(3, 5);
    }

    #[test]
    fn impl_for_vec() {
        assert_laws(vec![0, 1, 2, 3], vec![0, 2, 4, 6]);
    }
}
