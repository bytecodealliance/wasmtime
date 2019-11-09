pub use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
fn array_args(a: &mut [u8]) {
    a[0] = 1;
}

fn main() {
    let mut expected: &mut [u8] = &mut [0, 0];
    array_args(&mut expected);

    let given: &mut [u8] = &mut [0, 0];
    unsafe {
        wasi_common_array_args(given.as_mut_ptr(), given.len());
    }

    assert_eq!(given, expected);
}
