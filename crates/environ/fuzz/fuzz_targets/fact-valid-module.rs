//! A simple fuzzer for FACT
//!
//! This is an intentionally small fuzzer which is intended to only really be
//! used during the development of FACT itself when generating adapter modules.
//! This creates arbitrary adapter signatures and then generates the required
//! trampoline for that adapter ensuring that the final output wasm module is a
//! valid wasm module. This doesn't actually validate anything about the
//! correctness of the trampoline, only that it's valid wasm.

#![no_main]

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use wasmtime_environ::{ScopeVec, Tunables, component::*};
use wasmtime_test_util::component_fuzz::{MAX_TYPE_DEPTH, TestCase, Type};

const TYPE_COUNT: usize = 50;
const MAX_ARITY: u32 = 5;

#[derive(Debug)]
struct GenAdapter<'a> {
    test: TestCase<'a>,
    // TODO: Add these arbitrary options and thread them into
    // `Declarations::make_component`, or alternatively pass an `Unstructured`
    // into that method to make arbitrary choices for these things.
    //
    // post_return: bool,
    // lift_memory64: bool,
    // lower_memory64: bool,
}

fuzz_target!(|data: &[u8]| {
    let _ = target(data);
});

fn target(data: &[u8]) -> arbitrary::Result<()> {
    drop(env_logger::try_init());

    let mut u = Unstructured::new(data);

    // First generate a set of type to select from.
    let mut type_fuel = 1000;
    let mut types = Vec::new();
    for _ in 0..u.int_in_range(1..=TYPE_COUNT)? {
        // Only discount fuel if the generation was successful,
        // otherwise we'll get more random data and try again.
        types.push(Type::generate(&mut u, MAX_TYPE_DEPTH, &mut type_fuel)?);
    }

    // Next generate a static API test case driven by the above types.
    let mut params = Vec::new();
    let mut result = None;
    for _ in 0..u.int_in_range(0..=MAX_ARITY)? {
        params.push(u.choose(&types)?);
    }
    if u.arbitrary()? {
        result = Some(u.choose(&types)?);
    }

    let test = TestCase {
        params,
        result,
        encoding1: u.arbitrary()?,
        encoding2: u.arbitrary()?,
    };
    let adapter = GenAdapter { test };

    let wat_decls = adapter.test.declarations();
    let component = wat_decls.make_component();
    let component = wat::parse_str(&component).unwrap();

    let mut tunables = Tunables::default_host();
    tunables.debug_adapter_modules = u.arbitrary()?;

    let mut validator = wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all());
    let mut component_types = ComponentTypesBuilder::new(&validator);
    let adapters = ScopeVec::new();

    Translator::new(&tunables, &mut validator, &mut component_types, &adapters)
        .translate(&component)
        .expect("should never generate an invalid component");

    let adapters = adapters.into_iter();
    assert!(adapters.len() >= 1);
    for wasm in adapters {
        validator.reset();
        if let Err(err) = validator.validate_all(&wasm) {
            eprintln!("invalid wasm module: {err:?}");
            eprintln!("adapter: {adapter:?}");
            std::fs::write("invalid.wasm", &wasm).unwrap();
            match wasmprinter::print_bytes(&wasm) {
                Ok(s) => std::fs::write("invalid.wat", &s).unwrap(),
                Err(_) => drop(std::fs::remove_file("invalid.wat")),
            }
            panic!("invalid adapter: {err:?}")
        }
    }

    Ok(())
}
