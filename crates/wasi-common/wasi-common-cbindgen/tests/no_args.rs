pub use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
fn no_args() -> u32 {
    0
}

fn main() {
    assert_eq!(unsafe { wasi_common_no_args() }, no_args());
}
