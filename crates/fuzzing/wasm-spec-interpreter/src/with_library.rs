//! Interpret WebAssembly modules using the OCaml spec interpreter.
//!
//! ```
//! # use wasm_spec_interpreter::{SpecValue, interpret, instantiate};
//! let module = wat::parse_file("tests/add.wat").unwrap();
//! let instance = instantiate(&module).unwrap();
//! let parameters = vec![SpecValue::I32(42), SpecValue::I32(1)];
//! let results = interpret(&instance, "add", Some(parameters)).unwrap();
//! assert_eq!(results, &[SpecValue::I32(43)]);
//! ```
//!
//! ### Warning
//!
//! The OCaml runtime is [not re-entrant]. The code below must ensure that only
//! one Rust thread is executing at a time (using the `INTERPRET` lock) or we
//! may observe `SIGSEGV` failures, e.g., while running `cargo test`.
//!
//! [not re-entrant]:
//!     https://ocaml.org/manual/intfc.html#ss:parallel-execution-long-running-c-code
//!
//! ### Warning
//!
//! This module uses an unsafe approach (`OCamlRuntime::init_persistent()` +
//! `OCamlRuntime::recover_handle()`) to initializing the `OCamlRuntime` based
//! on some [discussion] with `ocaml-interop` crate authors. This approach was
//! their recommendation to resolve seeing errors like `boxroot is not setup`
//! followed by a `SIGSEGV`; this is similar to the testing approach [they use].
//! Use this approach with care and note that it is only as safe as the OCaml
//! code running underneath.
//!
//! [discussion]: https://github.com/tezedge/ocaml-interop/issues/35
//! [they use]:
//!     https://github.com/tezedge/ocaml-interop/blob/master/testing/rust-caller/src/lib.rs

use crate::{SpecExport, SpecInstance, SpecValue};
use ocaml_interop::{BoxRoot, OCamlRuntime, ToOCaml};
use std::sync::Mutex;

static INTERPRET: Mutex<()> = Mutex::new(());

/// Instantiate the WebAssembly module in the spec interpreter.
pub fn instantiate(module: &[u8]) -> Result<SpecInstance, String> {
    let _lock = INTERPRET.lock().unwrap();
    OCamlRuntime::init_persistent();
    let ocaml_runtime = unsafe { OCamlRuntime::recover_handle() };

    let module = module.to_boxroot(ocaml_runtime);
    let instance = ocaml_bindings::instantiate(ocaml_runtime, &module);
    instance.to_rust(ocaml_runtime)
}

/// Interpret the exported function `name` with the given `parameters`.
pub fn interpret(
    instance: &SpecInstance,
    name: &str,
    parameters: Option<Vec<SpecValue>>,
) -> Result<Vec<SpecValue>, String> {
    let _lock = INTERPRET.lock().unwrap();
    OCamlRuntime::init_persistent();
    let ocaml_runtime = unsafe { OCamlRuntime::recover_handle() };

    // Prepare the box-rooted parameters.
    let instance = instance.to_boxroot(ocaml_runtime);
    let name = name.to_string().to_boxroot(ocaml_runtime);
    let parameters = parameters.to_boxroot(ocaml_runtime);

    // Interpret the function.
    let results = ocaml_bindings::interpret(ocaml_runtime, &instance, &name, &parameters);
    results.to_rust(&ocaml_runtime)
}

/// Interpret the first function in the passed WebAssembly module (in Wasm form,
/// currently, not WAT), optionally with the given parameters. If no parameters
/// are provided, the function is invoked with zeroed parameters.
pub fn interpret_legacy(
    module: &[u8],
    opt_parameters: Option<Vec<SpecValue>>,
) -> Result<Vec<SpecValue>, String> {
    let _lock = INTERPRET.lock().unwrap();
    OCamlRuntime::init_persistent();
    let ocaml_runtime = unsafe { OCamlRuntime::recover_handle() };

    // Parse and execute, returning results converted to Rust.
    let module = module.to_boxroot(ocaml_runtime);
    let opt_parameters = opt_parameters.to_boxroot(ocaml_runtime);
    let results = ocaml_bindings::interpret_legacy(ocaml_runtime, &module, &opt_parameters);
    results.to_rust(ocaml_runtime)
}

/// Retrieve the export given by `name`.
pub fn export(instance: &SpecInstance, name: &str) -> Result<SpecExport, String> {
    let _lock = INTERPRET.lock().unwrap();
    OCamlRuntime::init_persistent();
    let ocaml_runtime = unsafe { OCamlRuntime::recover_handle() };

    // Prepare the box-rooted parameters.
    let instance = instance.to_boxroot(ocaml_runtime);
    let name = name.to_string().to_boxroot(ocaml_runtime);

    // Export the value.
    let results = ocaml_bindings::export(ocaml_runtime, &instance, &name);
    results.to_rust(&ocaml_runtime)
}

// Here we declare which functions we will use from the OCaml library. See
// https://docs.rs/ocaml-interop/0.8.4/ocaml_interop/index.html#example.
mod ocaml_bindings {
    use super::*;
    use ocaml_interop::{
        impl_conv_ocaml_variant, ocaml, FromOCaml, OCaml, OCamlBytes, OCamlInt32, OCamlInt64,
        OCamlList,
    };

    // Using this macro converts the enum both ways: Rust to OCaml and OCaml to
    // Rust. See
    // https://docs.rs/ocaml-interop/0.8.4/ocaml_interop/macro.impl_conv_ocaml_variant.html.
    impl_conv_ocaml_variant! {
        SpecValue {
            SpecValue::I32(i: OCamlInt32),
            SpecValue::I64(i: OCamlInt64),
            SpecValue::F32(i: OCamlInt32),
            SpecValue::F64(i: OCamlInt64),
            SpecValue::V128(i: OCamlBytes),
        }
    }

    // We need to also convert the `SpecExport` enum.
    impl_conv_ocaml_variant! {
        SpecExport {
            SpecExport::Global(i: SpecValue),
            SpecExport::Memory(i: OCamlBytes),
        }
    }

    // We manually show `SpecInstance` how to convert itself to and from OCaml.
    unsafe impl FromOCaml<SpecInstance> for SpecInstance {
        fn from_ocaml(v: OCaml<SpecInstance>) -> Self {
            Self {
                repr: BoxRoot::new(v),
            }
        }
    }
    unsafe impl ToOCaml<SpecInstance> for SpecInstance {
        fn to_ocaml<'a>(&self, cr: &'a mut OCamlRuntime) -> OCaml<'a, SpecInstance> {
            BoxRoot::get(&self.repr, cr)
        }
    }

    // These functions must be exposed from OCaml with:
    //  `Callback.register "interpret" interpret`
    //
    // In Rust, these functions look like:
    //   `pub fn interpret(_: &mut OCamlRuntime, ...: OCamlRef<...>) -> BoxRoot<...>;`
    //
    // The `ocaml!` macro does not understand documentation, so the
    // documentation is included here:
    // - `instantiate`: clear the global store and instantiate a new WebAssembly
    //   module from bytes
    // - `interpret`: given an instance, call the function exported at `name`
    // - `interpret_legacy`: starting from bytes, instantiate and execute the
    //   first exported function
    // - `export`: given an instance, get the value of the export at `name`
    ocaml! {
        pub fn instantiate(module: OCamlBytes) -> Result<SpecInstance, String>;
        pub fn interpret(instance: SpecInstance, name: String, params: Option<OCamlList<SpecValue>>) -> Result<OCamlList<SpecValue>, String>;
        pub fn interpret_legacy(module: OCamlBytes, params: Option<OCamlList<SpecValue>>) -> Result<OCamlList<SpecValue>, String>;
        pub fn export(instance: SpecInstance, name: String) -> Result<SpecExport, String>;
    }
}

/// Initialize a persistent OCaml runtime.
///
/// When used for fuzzing differentially with engines that also use signal
/// handlers, this function provides a way to explicitly set up the OCaml
/// runtime and configure its signal handlers.
pub fn setup_ocaml_runtime() {
    let _lock = INTERPRET.lock().unwrap();
    OCamlRuntime::init_persistent();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_function_name() {
        let module = wat::parse_file("tests/add.wat").unwrap();
        let instance = instantiate(&module).unwrap();
        let results = interpret(
            &instance,
            "not-the-right-name",
            Some(vec![SpecValue::I32(0), SpecValue::I32(0)]),
        );
        assert_eq!(results, Err("Not_found".to_string()));
    }

    #[test]
    fn multiple_invocation() {
        let module = wat::parse_file("tests/add.wat").unwrap();
        let instance = instantiate(&module).unwrap();

        let results1 = interpret(
            &instance,
            "add",
            Some(vec![SpecValue::I32(42), SpecValue::I32(1)]),
        )
        .unwrap();
        let results2 = interpret(
            &instance,
            "add",
            Some(vec![SpecValue::I32(1), SpecValue::I32(42)]),
        )
        .unwrap();
        assert_eq!(results1, results2);

        let results3 = interpret(
            &instance,
            "add",
            Some(vec![SpecValue::I32(20), SpecValue::I32(23)]),
        )
        .unwrap();
        assert_eq!(results2, results3);
    }

    #[test]
    fn multiple_invocation_legacy() {
        let module = wat::parse_file("tests/add.wat").unwrap();

        let results1 =
            interpret_legacy(&module, Some(vec![SpecValue::I32(42), SpecValue::I32(1)])).unwrap();
        let results2 =
            interpret_legacy(&module, Some(vec![SpecValue::I32(1), SpecValue::I32(42)])).unwrap();
        assert_eq!(results1, results2);

        let results3 =
            interpret_legacy(&module, Some(vec![SpecValue::I32(20), SpecValue::I32(23)])).unwrap();
        assert_eq!(results2, results3);
    }

    #[test]
    fn oob() {
        let module = wat::parse_file("tests/oob.wat").unwrap();
        let instance = instantiate(&module).unwrap();
        let results = interpret(&instance, "oob", None);
        assert_eq!(
            results,
            Err("Error(_, \"(Isabelle) trap: load\")".to_string())
        );
    }

    #[test]
    fn oob_legacy() {
        let module = wat::parse_file("tests/oob.wat").unwrap();
        let results = interpret_legacy(&module, None);
        assert_eq!(
            results,
            Err("Error(_, \"(Isabelle) trap: load\")".to_string())
        );
    }

    #[test]
    fn simd_not() {
        let module = wat::parse_file("tests/simd_not.wat").unwrap();
        let instance = instantiate(&module).unwrap();

        let parameters = Some(vec![SpecValue::V128(vec![
            0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0,
        ])]);
        let results = interpret(&instance, "simd_not", parameters).unwrap();

        assert_eq!(
            results,
            vec![SpecValue::V128(vec![
                255, 0, 255, 255, 0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 255, 255
            ])]
        );
    }

    #[test]
    fn simd_not_legacy() {
        let module = wat::parse_file("tests/simd_not.wat").unwrap();

        let parameters = Some(vec![SpecValue::V128(vec![
            0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0,
        ])]);
        let results = interpret_legacy(&module, parameters).unwrap();

        assert_eq!(
            results,
            vec![SpecValue::V128(vec![
                255, 0, 255, 255, 0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 255, 255
            ])]
        );
    }

    // See issue https://github.com/bytecodealliance/wasmtime/issues/4671.
    #[test]
    fn order_of_params() {
        let module = wat::parse_file("tests/shr_s.wat").unwrap();
        let instance = instantiate(&module).unwrap();

        let parameters = Some(vec![
            SpecValue::I32(1795123818),
            SpecValue::I32(-2147483648),
        ]);
        let results = interpret(&instance, "test", parameters).unwrap();

        assert_eq!(results, vec![SpecValue::I32(1795123818)]);
    }

    // See issue https://github.com/bytecodealliance/wasmtime/issues/4671.
    #[test]
    fn order_of_params_legacy() {
        let module = wat::parse_file("tests/shr_s.wat").unwrap();

        let parameters = Some(vec![
            SpecValue::I32(1795123818),
            SpecValue::I32(-2147483648),
        ]);
        let results = interpret_legacy(&module, parameters).unwrap();

        assert_eq!(results, vec![SpecValue::I32(1795123818)]);
    }

    #[test]
    fn load_store_and_export() {
        let module = wat::parse_file("tests/memory.wat").unwrap();
        let instance = instantiate(&module).unwrap();

        // Store 42 at offset 4.
        let _ = interpret(
            &instance,
            "store_i32",
            Some(vec![SpecValue::I32(4), SpecValue::I32(42)]),
        );

        // Load an i32 from offset 4.
        let loaded = interpret(&instance, "load_i32", Some(vec![SpecValue::I32(4)]));

        // Check stored value was retrieved.
        assert_eq!(loaded.unwrap(), vec![SpecValue::I32(42)]);

        // Retrieve the memory exported with name "mem" and check that the
        // 32-bit value at byte offset 4 of memory is 42.
        let export = export(&instance, "mem");
        match export.unwrap() {
            SpecExport::Global(_) => panic!("incorrect export"),
            SpecExport::Memory(m) => {
                assert_eq!(&m[0..10], [0, 0, 0, 0, 42, 0, 0, 0, 0, 0]);
            }
        }
    }
}
