use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::{Engine, Store};

#[test]
fn into_inner() {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let engine = Engine::default();
    assert_eq!(HITS.load(SeqCst), 0);
    drop(Store::new(&engine, A));
    assert_eq!(HITS.load(SeqCst), 1);
    Store::new(&engine, A).into_data();
    assert_eq!(HITS.load(SeqCst), 2);
}
