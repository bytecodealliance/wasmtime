use std::collections::HashMap;
use std::hash::Hash;

pub trait MapWithDefault<K, V: Default> {
    fn get_or_default(&mut self, k: K) -> &mut V;
}

impl<K: Eq + Hash, V: Default> MapWithDefault<K, V> for HashMap<K, V> {
    fn get_or_default(&mut self, k: K) -> &mut V {
        self.entry(k).or_insert_with(|| V::default())
    }
}

#[test]
fn test_default() {
    let mut hash_map = HashMap::new();
    hash_map.insert(42, "hello");
    assert_eq!(*hash_map.get_or_default(43), "");
}
