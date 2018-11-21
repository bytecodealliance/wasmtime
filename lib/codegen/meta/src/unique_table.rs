use std::slice;

/// A table of sequences which tries to avoid common subsequences.
pub struct UniqueTable<T: PartialEq + Clone> {
    table: Vec<T>,
}

impl<T: PartialEq + Clone> UniqueTable<T> {
    pub fn new() -> Self {
        Self { table: Vec::new() }
    }
    pub fn add(&mut self, values: &Vec<T>) -> usize {
        if let Some(offset) = find_subsequence(values, &self.table) {
            offset
        } else {
            let offset = self.table.len();
            self.table.extend((*values).clone());
            offset
        }
    }
    pub fn len(&self) -> usize {
        self.table.len()
    }
    pub fn iter(&self) -> slice::Iter<T> {
        self.table.iter()
    }
}

/// Try to find the subsequence `sub` in the `whole` sequence. Returns None if
/// it's not been found, or Some(index) if it has been. Naive implementation
/// until proven we need something better.
fn find_subsequence<T: PartialEq>(sub: &Vec<T>, whole: &Vec<T>) -> Option<usize> {
    assert!(sub.len() > 0);
    // We want i + sub.len() <= whole.len(), i.e. i < whole.len() + 1 - sub.len().
    if whole.len() < sub.len() {
        return None;
    }
    let max = whole.len() + 1 - sub.len();
    for i in 0..max {
        let mut found: Option<usize> = Some(i);
        for j in 0..sub.len() {
            if sub[j] != whole[i + j] {
                found = None;
                break;
            }
        }
        if found.is_some() {
            return found;
        }
    }
    return None;
}

#[test]
fn test_find_subsequence() {
    assert_eq!(find_subsequence(&vec![1], &vec![4]), None);
    assert_eq!(find_subsequence(&vec![1], &vec![1]), Some(0));
    assert_eq!(find_subsequence(&vec![1, 2], &vec![1]), None);
    assert_eq!(find_subsequence(&vec![1, 2], &vec![1, 2]), Some(0));
    assert_eq!(find_subsequence(&vec![1, 2], &vec![1, 3]), None);
    assert_eq!(find_subsequence(&vec![1, 2], &vec![0, 1, 2]), Some(1));
    assert_eq!(find_subsequence(&vec![1, 2], &vec![0, 1, 3, 1]), None);
    assert_eq!(find_subsequence(&vec![1, 2], &vec![0, 1, 3, 1, 2]), Some(3));
    assert_eq!(
        find_subsequence(&vec![1, 1, 3], &vec![1, 1, 1, 3, 3]),
        Some(1)
    );
}
