pub use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
fn mut_args(a: &mut usize) {
    *a = *a + 1
}

fn main() {
    let mut expected = Box::new(2);
    mut_args(expected.as_mut());
    let given = unsafe {
        let given = Box::new(2);
        let raw = Box::into_raw(given);
        wasi_common_mut_args(raw);
        Box::from_raw(raw)
    };
    assert_eq!(*given, *expected);
}
