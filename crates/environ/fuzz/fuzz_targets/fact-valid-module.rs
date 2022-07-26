//! A simple fuzzer for FACT
//!
//! This is an intentionally small fuzzer which is intended to only really be
//! used during the development of FACT itself when generating adapter modules.
//! This creates arbitrary adapter signatures and then generates the required
//! trampoline for that adapter ensuring that the final output wasm module is a
//! valid wasm module. This doesn't actually validate anything about the
//! correctness of the trampoline, only that it's valid wasm.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::fmt;
use wasmparser::{Validator, WasmFeatures};
use wasmtime_environ::component::*;
use wasmtime_environ::fact::Module;

#[derive(Arbitrary, Debug)]
struct GenAdapterModule {
    debug: bool,
    adapters: Vec<GenAdapter>,
}

#[derive(Arbitrary, Debug)]
struct GenAdapter {
    ty: FuncType,
    post_return: bool,
}

#[derive(Arbitrary, Debug)]
struct FuncType {
    params: Vec<ValType>,
    result: ValType,
}

#[derive(Arbitrary, Debug)]
enum ValType {
    Unit,
    U8,
    S8,
    U16,
    S16,
    U32,
    S32,
    U64,
    S64,
    Float32,
    Float64,
    Record(Vec<ValType>),
    Tuple(Vec<ValType>),
    Variant(NonZeroLenVec<ValType>),
}

pub struct NonZeroLenVec<T>(Vec<T>);

impl<'a, T: Arbitrary<'a>> Arbitrary<'a> for NonZeroLenVec<T> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut items = Vec::arbitrary(u)?;
        if items.is_empty() {
            items.push(u.arbitrary()?);
        }
        Ok(NonZeroLenVec(items))
    }
}

impl<T: fmt::Debug> fmt::Debug for NonZeroLenVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

fuzz_target!(|module: GenAdapterModule| {
    drop(env_logger::try_init());

    let mut types = ComponentTypesBuilder::default();

    let mut next_def = 0;
    let mut dummy_def = || {
        next_def += 1;
        CoreDef::Adapter(AdapterIndex::from_u32(next_def))
    };
    let mut next_memory = 0;
    let mut dummy_memory = || {
        // Limit the number of memory imports generated since `wasmparser` has a
        // hardcoded limit of ~100 for now anyway.
        if next_memory < 20 {
            next_memory += 1;
        }
        CoreExport {
            instance: RuntimeInstanceIndex::from_u32(next_memory),
            item: ExportItem::Name(String::new()),
        }
    };

    let mut adapters = Vec::new();
    for adapter in module.adapters.iter() {
        let mut params = Vec::new();
        for param in adapter.ty.params.iter() {
            params.push((None, intern(&mut types, param)));
        }
        let result = intern(&mut types, &adapter.ty.result);
        let signature = types.add_func_type(TypeFunc {
            params: params.into(),
            result,
        });
        adapters.push(Adapter {
            lift_ty: signature,
            lower_ty: signature,
            lower_options: AdapterOptions {
                instance: RuntimeComponentInstanceIndex::from_u32(0),
                string_encoding: StringEncoding::Utf8,
                // Pessimistically assume that memory/realloc are going to be
                // required for this trampoline and provide it. Avoids doing
                // calculations to figure out whether they're necessary and
                // simplifies the fuzzer here without reducing coverage within FACT
                // itself.
                memory: Some(dummy_memory()),
                realloc: Some(dummy_def()),
                // Lowering never allows `post-return`
                post_return: None,
            },
            lift_options: AdapterOptions {
                instance: RuntimeComponentInstanceIndex::from_u32(1),
                string_encoding: StringEncoding::Utf8,
                memory: Some(dummy_memory()),
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
    let types = types.finish();
    let mut module = Module::new(&types, module.debug);
    for (i, adapter) in adapters.iter().enumerate() {
        module.adapt(&format!("adapter{i}"), adapter);
    }
    let wasm = module.encode();
    let result = Validator::new_with_features(WasmFeatures {
        multi_memory: true,
        ..WasmFeatures::default()
    })
    .validate_all(&wasm);

    let err = match result {
        Ok(_) => return,
        Err(e) => e,
    };
    eprintln!("invalid wasm module: {err:?}");
    for adapter in adapters.iter() {
        eprintln!("adapter type: {:?}", types[adapter.lift_ty]);
    }
    std::fs::write("invalid.wasm", &wasm).unwrap();
    match wasmprinter::print_bytes(&wasm) {
        Ok(s) => std::fs::write("invalid.wat", &s).unwrap(),
        Err(_) => drop(std::fs::remove_file("invalid.wat")),
    }

    panic!()
});

fn intern(types: &mut ComponentTypesBuilder, ty: &ValType) -> InterfaceType {
    match ty {
        ValType::Unit => InterfaceType::Unit,
        ValType::U8 => InterfaceType::U8,
        ValType::S8 => InterfaceType::S8,
        ValType::U16 => InterfaceType::U16,
        ValType::S16 => InterfaceType::S16,
        ValType::U32 => InterfaceType::U32,
        ValType::S32 => InterfaceType::S32,
        ValType::U64 => InterfaceType::U64,
        ValType::S64 => InterfaceType::S64,
        ValType::Float32 => InterfaceType::Float32,
        ValType::Float64 => InterfaceType::Float64,
        ValType::Record(tys) => {
            let ty = TypeRecord {
                fields: tys
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| RecordField {
                        name: format!("f{i}"),
                        ty: intern(types, ty),
                    })
                    .collect(),
            };
            InterfaceType::Record(types.add_record_type(ty))
        }
        ValType::Tuple(tys) => {
            let ty = TypeTuple {
                types: tys.iter().map(|ty| intern(types, ty)).collect(),
            };
            InterfaceType::Tuple(types.add_tuple_type(ty))
        }
        ValType::Variant(NonZeroLenVec(cases)) => {
            let ty = TypeVariant {
                cases: cases
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| VariantCase {
                        name: format!("c{i}"),
                        ty: intern(types, ty),
                    })
                    .collect(),
            };
            InterfaceType::Variant(types.add_variant_type(ty))
        }
    }
}
