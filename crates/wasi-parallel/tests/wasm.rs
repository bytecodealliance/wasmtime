//! Run the test cases in the `wasm` directory.
//!
//! See the `build.rs` file for how these WebAssembly binaries are generated.

mod test_case;

#[cfg(feature = "build-tests")]
#[test]
fn run_buffer_rs() {
    let mut test_case =
        test_case::TestCase::new("tests/wasm/buffer.wasm", test_case::default_engine(), None)
            .unwrap();
    let results = test_case.invoke("main", &[0.into(), 0.into()]).unwrap();
    assert_eq!(results[0].i32().unwrap(), 0);
}
