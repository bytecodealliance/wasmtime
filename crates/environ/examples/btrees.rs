use std::{collections::BTreeMap, str::FromStr};
use wasmtime_environ::{
    collections,
    error::{Context as _, Result, bail, format_err},
};

type Key = [u128; 2];
type Value = [u8; 16];

trait Map {
    fn insert(&mut self, key: Key, value: Value) -> Result<()>;
}

impl Map for BTreeMap<Key, Value> {
    fn insert(&mut self, key: Key, value: Value) -> Result<()> {
        BTreeMap::insert(self, key, value);
        Ok(())
    }
}

impl Map for collections::TryBTreeMap<Key, Value> {
    fn insert(&mut self, key: Key, value: Value) -> Result<()> {
        collections::TryBTreeMap::insert(self, key, value)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let kind = std::env::args()
        .nth(1)
        .ok_or_else(|| format_err!("must provide first argument: 'std' or 'bforest'"))?;

    let mut map: Box<dyn Map> = match kind.as_str() {
        "std" => Box::new(BTreeMap::new()),
        "bforest" => Box::new(collections::TryBTreeMap::new()),
        _ => bail!("first argument must be either 'std' or 'bforest', got: '{kind}'"),
    };

    let n = std::env::args().nth(2);
    let n = n.as_deref().unwrap_or("1000");
    let n = u128::from_str(n).context("failed to parse second argument as `u32` integer")?;

    println!("Inserting {n} entries into `{kind}`-based `BTreeMap`...");

    for i in 0..n {
        map.insert([i, i], [0; 16])?;
    }

    Ok(())
}
