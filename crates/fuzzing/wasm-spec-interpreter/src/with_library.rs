//! Interpret WebAssembly modules using the OCaml spec interpreter.
//! ```
//! # use wasm_spec_interpreter::{Value, interpret};
//! let module = wat::parse_file("tests/add.wat").unwrap();
//! let parameters = vec![Value::I32(42), Value::I32(1)];
//! let results = interpret(&module, Some(parameters)).unwrap();
//! assert_eq!(results, &[Value::I32(43)]);
//! ```
use crate::Value;
use lazy_static::lazy_static;
use ocaml_interop::{OCamlRuntime, ToOCaml};
use std::sync::Mutex;

lazy_static! {
    static ref INTERPRET: Mutex<()> = Mutex::new(());
}

/// Interpret the first function in the passed WebAssembly module (in Wasm form,
/// currently, not WAT), optionally with the given parameters. If no parameters
/// are provided, the function is invoked with zeroed parameters.
pub fn interpret(module: &[u8], opt_parameters: Option<Vec<Value>>) -> Result<Vec<Value>, String> {
    // The OCaml runtime is not re-entrant
    // (https://ocaml.org/manual/intfc.html#ss:parallel-execution-long-running-c-code).
    // We need  to make sure that only one Rust thread is executing at a time
    // (using this lock) or we can observe `SIGSEGV` failures while running
    // `cargo test`.
    let _lock = INTERPRET.lock().unwrap();
    // Here we use an unsafe approach to initializing the `OCamlRuntime` based
    // on the discussion in https://github.com/tezedge/ocaml-interop/issues/35.
    // This was the recommendation to resolve seeing errors like `boxroot is not
    // setup` followed by a `SIGSEGV`; this is similar to the testing approach
    // in
    // https://github.com/tezedge/ocaml-interop/blob/master/testing/rust-caller/src/lib.rs
    // and is only as safe as the OCaml code running underneath.
    OCamlRuntime::init_persistent();
    let ocaml_runtime = unsafe { OCamlRuntime::recover_handle() };
    // Parse and execute, returning results converted to Rust.
    let module = module.to_boxroot(ocaml_runtime);

    let opt_parameters = opt_parameters.to_boxroot(ocaml_runtime);
    let results = ocaml_bindings::interpret(ocaml_runtime, &module, &opt_parameters);
    results.to_rust(ocaml_runtime)
}

// Here we declare which functions we will use from the OCaml library. See
// https://docs.rs/ocaml-interop/0.8.4/ocaml_interop/index.html#example.
mod ocaml_bindings {
    use super::*;
    use ocaml_interop::{
        impl_conv_ocaml_variant, ocaml, OCamlBytes, OCamlInt32, OCamlInt64, OCamlList,
    };

    // Using this macro converts the enum both ways: Rust to OCaml and OCaml to
    // Rust. See
    // https://docs.rs/ocaml-interop/0.8.4/ocaml_interop/macro.impl_conv_ocaml_variant.html.
    impl_conv_ocaml_variant! {
        Value {
            Value::I32(i: OCamlInt32),
            Value::I64(i: OCamlInt64),
            Value::F32(i: OCamlInt32),
            Value::F64(i: OCamlInt64),
        }
    }

    // These functions must be exposed from OCaml with:
    //   `Callback.register "interpret" interpret`
    //
    // In Rust, this function becomes:
    //   `pub fn interpret(_: &mut OCamlRuntime, ...: OCamlRef<...>) -> BoxRoot<...>;`
    ocaml! {
        pub fn interpret(module: OCamlBytes, params: Option<OCamlList<Value>>) -> Result<OCamlList<Value>, String>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple() {
        let module = wat::parse_file("tests/add.wat").unwrap();

        let parameters1 = Some(vec![Value::I32(42), Value::I32(1)]);
        let results1 = interpret(&module, parameters1.clone()).unwrap();

        let parameters2 = Some(vec![Value::I32(1), Value::I32(42)]);
        let results2 = interpret(&module, parameters2.clone()).unwrap();

        assert_eq!(results1, results2);

        let parameters3 = Some(vec![Value::I32(20), Value::I32(23)]);
        let results3 = interpret(&module, parameters3.clone()).unwrap();

        assert_eq!(results2, results3);
    }

    #[test]
    fn oob() {
        let module = wat::parse_file("tests/oob.wat").unwrap();
        let results = interpret(&module, None);
        assert_eq!(
            results,
            Err("Error(_, \"(Isabelle) trap: load\")".to_string())
        );
    }
}
