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
    lift_memory64: bool,
    lower_memory64: bool,
    lift_encoding: GenStringEncoding,
    lower_encoding: GenStringEncoding,
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
    Char,
    Record(Vec<ValType>),
    // FIXME(WebAssembly/component-model#75) are zero-sized flags allowed?
    //
    // ... otherwise go up to 65 flags to exercise up to 3 u32 values
    Flags(UsizeInRange<1, 65>),
    Tuple(Vec<ValType>),
    Variant(NonZeroLenVec<ValType>),
    Union(NonZeroLenVec<ValType>),
    // at least one enum variant but no more than what's necessary to inflate to
    // 16 bits to keep this reasonably sized
    Enum(UsizeInRange<1, 257>),
    Option(Box<ValType>),
    Expected(Box<ValType>, Box<ValType>),
}

#[derive(Copy, Clone, Arbitrary, Debug)]
enum GenStringEncoding {
    Utf8,
    Utf16,
    CompactUtf16,
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

pub struct UsizeInRange<const L: usize, const H: usize>(usize);

impl<'a, const L: usize, const H: usize> Arbitrary<'a> for UsizeInRange<L, H> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(UsizeInRange(u.int_in_range(L..=H)?))
    }
}

impl<const L: usize, const H: usize> fmt::Debug for UsizeInRange<L, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

fuzz_target!(|module: GenAdapterModule| {
    drop(env_logger::try_init());

    let mut types = ComponentTypesBuilder::default();

    // Manufactures a unique `CoreDef` so all function imports get unique
    // function imports.
    let mut next_def = 0;
    let mut dummy_def = || {
        next_def += 1;
        CoreDef::Adapter(AdapterIndex::from_u32(next_def))
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
        CoreExport {
            instance: RuntimeInstanceIndex::from_u32(idx),
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
                string_encoding: adapter.lower_encoding.into(),
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
                string_encoding: adapter.lift_encoding.into(),
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
    let types = types.finish();
    let mut fact_module = Module::new(&types, module.debug);
    for (i, adapter) in adapters.iter().enumerate() {
        fact_module.adapt(&format!("adapter{i}"), adapter);
    }
    let wasm = fact_module.encode();
    let result = Validator::new_with_features(WasmFeatures {
        multi_memory: true,
        memory64: true,
        ..WasmFeatures::default()
    })
    .validate_all(&wasm);

    let err = match result {
        Ok(_) => return,
        Err(e) => e,
    };
    eprintln!("invalid wasm module: {err:?}");
    for adapter in module.adapters.iter() {
        eprintln!("adapter: {adapter:?}");
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
        ValType::Char => InterfaceType::Char,
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
        ValType::Flags(size) => {
            let ty = TypeFlags {
                names: (0..size.0).map(|i| format!("f{i}")).collect(),
            };
            InterfaceType::Flags(types.add_flags_type(ty))
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
        ValType::Union(tys) => {
            let ty = TypeUnion {
                types: tys.0.iter().map(|ty| intern(types, ty)).collect(),
            };
            InterfaceType::Union(types.add_union_type(ty))
        }
        ValType::Enum(size) => {
            let ty = TypeEnum {
                names: (0..size.0).map(|i| format!("c{i}")).collect(),
            };
            InterfaceType::Enum(types.add_enum_type(ty))
        }
        ValType::Option(ty) => {
            let ty = intern(types, ty);
            InterfaceType::Option(types.add_interface_type(ty))
        }
        ValType::Expected(ok, err) => {
            let ok = intern(types, ok);
            let err = intern(types, err);
            InterfaceType::Expected(types.add_expected_type(TypeExpected { ok, err }))
        }
    }
}

impl From<GenStringEncoding> for StringEncoding {
    fn from(gen: GenStringEncoding) -> StringEncoding {
        match gen {
            GenStringEncoding::Utf8 => StringEncoding::Utf8,
            GenStringEncoding::Utf16 => StringEncoding::Utf16,
            GenStringEncoding::CompactUtf16 => StringEncoding::CompactUtf16,
        }
    }
}
