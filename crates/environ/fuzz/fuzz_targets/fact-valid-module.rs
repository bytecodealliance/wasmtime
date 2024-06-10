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
use component_fuzz_util::{TestCase, Type, MAX_TYPE_DEPTH};
use libfuzzer_sys::fuzz_target;
use wasmparser::types::ComponentAnyTypeId;
use wasmparser::{Parser, Payload, Validator, WasmFeatures};
use wasmtime_environ::component::*;
use wasmtime_environ::fact::Module;

const TYPE_COUNT: usize = 50;
const MAX_ARITY: u32 = 5;
const TEST_CASE_COUNT: usize = 20;

#[derive(Debug)]
struct GenAdapterModule<'a> {
    debug: bool,
    adapters: Vec<GenAdapter<'a>>,
}

#[derive(Debug)]
struct GenAdapter<'a> {
    post_return: bool,
    lift_memory64: bool,
    lower_memory64: bool,
    test: TestCase<'a>,
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

    // Next generate a set of static API test cases driven by the above
    // types.
    let mut ret = GenAdapterModule {
        debug: u.arbitrary()?,
        adapters: Vec::new(),
    };
    for _ in 0..u.int_in_range(1..=TEST_CASE_COUNT)? {
        let mut params = Vec::new();
        let mut results = Vec::new();
        for _ in 0..u.int_in_range(0..=MAX_ARITY)? {
            params.push(u.choose(&types)?);
        }
        for _ in 0..u.int_in_range(0..=MAX_ARITY)? {
            results.push(u.choose(&types)?);
        }

        let test = TestCase {
            params,
            results,
            encoding1: u.arbitrary()?,
            encoding2: u.arbitrary()?,
        };
        ret.adapters.push(GenAdapter {
            test,
            post_return: u.arbitrary()?,
            lift_memory64: u.arbitrary()?,
            lower_memory64: u.arbitrary()?,
        });
    }

    // Manufactures a unique `CoreDef` so all function imports get unique
    // function imports.
    let mut next_def = 0;
    let mut dummy_def = || {
        next_def += 1;
        dfg::CoreDef::Adapter(dfg::AdapterId::from_u32(next_def))
    };

    // Manufactures a `CoreExport` for a memory with the shape specified. Note
    // that we can't import as many memories as functions so these are
    // intentionally limited. Once a handful of memories are generated of each
    // type then they start getting reused.
    let mut next_memory = 0;
    let mut memories32 = Vec::new();
    let mut memories64 = Vec::new();
    let mut dummy_memory = |memory64: bool| {
        let dst = if memory64 {
            &mut memories64
        } else {
            &mut memories32
        };
        let idx = if dst.len() < 5 {
            next_memory += 1;
            dst.push(next_memory - 1);
            next_memory - 1
        } else {
            dst[0]
        };
        dfg::CoreExport {
            instance: dfg::InstanceId::from_u32(idx),
            item: ExportItem::Name(String::new()),
        }
    };
    let mut validator = Validator::new();
    let mut types = ComponentTypesBuilder::new(&validator);

    let mut adapters = Vec::new();
    for adapter in ret.adapters.iter() {
        let wat_decls = adapter.test.declarations();
        let wat = format!(
            "(component
                {types}
                (type (func {params} {results}))
            )",
            types = wat_decls.types,
            params = wat_decls.params,
            results = wat_decls.results,
        );
        let wasm = wat::parse_str(&wat).unwrap();

        let mut type_index = 0;
        for payload in Parser::new(0).parse_all(&wasm) {
            let payload = payload.unwrap();
            validator.payload(&payload).unwrap();
            let section = match payload {
                Payload::ComponentTypeSection(s) => s,
                _ => continue,
            };
            for _ in section {
                let validator_types = validator.types(0).unwrap();
                let id = validator_types.component_any_type_at(type_index);
                type_index += 1;
                let id = match id {
                    ComponentAnyTypeId::Func(id) => id,
                    _ => continue,
                };
                let ty = types
                    .convert_component_func_type(validator_types, id)
                    .unwrap();
                adapters.push(Adapter {
                    lift_ty: ty,
                    lower_ty: ty,
                    lower_options: AdapterOptions {
                        instance: RuntimeComponentInstanceIndex::from_u32(0),
                        string_encoding: convert_encoding(adapter.test.encoding1),
                        memory64: adapter.lower_memory64,
                        // Pessimistically assume that memory/realloc are going to be
                        // required for this trampoline and provide it. Avoids doing
                        // calculations to figure out whether they're necessary and
                        // simplifies the fuzzer here without reducing coverage within FACT
                        // itself.
                        memory: Some(dummy_memory(adapter.lower_memory64)),
                        realloc: Some(dummy_def()),
                        // Lowering never allows `post-return`
                        post_return: None,
                    },
                    lift_options: AdapterOptions {
                        instance: RuntimeComponentInstanceIndex::from_u32(1),
                        string_encoding: convert_encoding(adapter.test.encoding2),
                        memory64: adapter.lift_memory64,
                        memory: Some(dummy_memory(adapter.lift_memory64)),
                        realloc: Some(dummy_def()),
                        post_return: if adapter.post_return {
                            Some(dummy_def())
                        } else {
                            None
                        },
                    },
                    func: dummy_def(),
                });
            }
        }
        validator.reset();
    }

    let mut fact_module = Module::new(&types, ret.debug);
    for (i, adapter) in adapters.iter().enumerate() {
        fact_module.adapt(&format!("adapter{i}"), adapter);
    }
    let wasm = fact_module.encode();
    let result = Validator::new_with_features(WasmFeatures::default() | WasmFeatures::MEMORY64)
        .validate_all(&wasm);

    let err = match result {
        Ok(_) => return Ok(()),
        Err(e) => e,
    };
    eprintln!("invalid wasm module: {err:?}");
    for adapter in ret.adapters.iter() {
        eprintln!("adapter: {adapter:?}");
    }
    std::fs::write("invalid.wasm", &wasm).unwrap();
    match wasmprinter::print_bytes(&wasm) {
        Ok(s) => std::fs::write("invalid.wat", &s).unwrap(),
        Err(_) => drop(std::fs::remove_file("invalid.wat")),
    }

    panic!()
}

fn convert_encoding(encoding: component_fuzz_util::StringEncoding) -> StringEncoding {
    match encoding {
        component_fuzz_util::StringEncoding::Utf8 => StringEncoding::Utf8,
        component_fuzz_util::StringEncoding::Utf16 => StringEncoding::Utf16,
        component_fuzz_util::StringEncoding::Latin1OrUtf16 => StringEncoding::CompactUtf16,
    }
}
