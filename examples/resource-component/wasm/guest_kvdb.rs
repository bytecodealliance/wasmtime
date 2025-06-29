wit_bindgen::generate!({
    path: "..",
    world: "kv-database",
});

use crate::example::kv_store::kvdb::Connection;
use std::sync::LazyLock;

static KV_CONNECTION: LazyLock<Connection> = LazyLock::new(Connection::new);

struct KVStore;

impl Guest for KVStore {
    // implement the guest function
    fn replace_value(key: String, value: String) -> Option<String> {
        // replace
        let old = KV_CONNECTION.get(&key);
        KV_CONNECTION.set(&key, &value);
        old
    }
}

export!(KVStore);
