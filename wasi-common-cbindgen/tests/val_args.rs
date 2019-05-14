extern crate wasi_common_cbindgen;

pub use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
fn val_args(a: usize, b: usize) -> usize {
    a + b
}

fn main() {
    assert_eq!(unsafe { __wasi_val_args(1, 2) }, val_args(1, 2));
}