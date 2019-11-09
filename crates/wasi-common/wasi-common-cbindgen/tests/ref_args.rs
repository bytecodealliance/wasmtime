pub use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
fn ref_args(a: &usize) -> usize {
    a + 1
}

fn main() {
    let a = Box::new(2);
    let expected = ref_args(a.as_ref());
    let given = unsafe {
        let raw = Box::into_raw(a);
        let res = wasi_common_ref_args(raw);
        Box::from_raw(raw);
        res
    };
    assert_eq!(given, expected);
}
