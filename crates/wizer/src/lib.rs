//! Wizer: the WebAssembly initializer!
//!
//! See the [`Wizer`] struct for details.

#![deny(missing_docs)]

use anyhow::Context;
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use std::convert::TryFrom;
#[cfg(feature = "structopt")]
use structopt::StructOpt;

const WASM_PAGE_SIZE: u32 = 65_536;
const NATIVE_PAGE_SIZE: u32 = 4_096;

/// Wizer: the WebAssembly initializer!
///
/// Don't wait for your Wasm module to initialize itself, pre-initialize it!
/// Wizer instantiates your WebAssembly module, executes its initialization
/// function, and then serializes the instance's initialized state out into a
/// new WebAssembly module. Now you can use this new, pre-initialized
/// WebAssembly module to hit the ground running, without making your users wait
/// for that first-time set up code to complete.
///
/// ## Caveats
///
/// * The initialization function may not call any imported functions. Doing so
///   will trigger a trap and `wizer` will exit.
///
/// * The Wasm module may not import globals, tables, or memories.
///
/// * Reference types are not supported yet. This is tricky because it would
///   allow the Wasm module to mutate tables, and we would need to be able to
///   diff the initial table state with the new table state, but funcrefs and
///   externrefs aren't comparable in the Wasm spec, which makes diffing
///   problematic.
#[cfg_attr(feature = "structopt", derive(StructOpt))]
#[derive(Clone, Debug)]
pub struct Wizer {
    /// The Wasm export name of the function that should be executed to
    /// initialize the Wasm module.
    #[cfg_attr(
        feature = "structopt",
        structopt(short = "f", long = "init-func", default_value = "wizer.initialize")
    )]
    init_func: String,

    /// Allow WASI imports to be called during initialization.
    ///
    /// This can introduce diverging semantics because the initialization can
    /// observe nondeterminism that might have gone a different way at runtime
    /// than it did at initialization time.
    ///
    /// If your Wasm module uses WASI's `get_random` to add randomness to
    /// something as a security mitigation (e.g. something akin to ASLR or the
    /// way Rust's hash maps incorporate a random nonce) then note that, if the
    /// randomization is added during initialization time and you don't ever
    /// re-randomize at runtime, then that randomization will become per-module
    /// rather than per-instance.
    #[cfg_attr(feature = "structopt", structopt(long = "allow-wasi"))]
    allow_wasi: bool,
}

fn translate_val_type(ty: wasmparser::Type) -> wasm_encoder::ValType {
    use wasm_encoder::ValType;
    use wasmparser::Type::*;
    match ty {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        V128 | FuncRef | ExternRef | ExnRef => panic!("not supported"),
        Func | EmptyBlockType => unreachable!(),
    }
}

fn translate_global_type(ty: wasmparser::GlobalType) -> wasm_encoder::GlobalType {
    wasm_encoder::GlobalType {
        val_type: translate_val_type(ty.content_type),
        mutable: ty.mutable,
    }
}

impl Wizer {
    /// Construct a new `Wizer` builder.
    pub fn new() -> Self {
        Wizer {
            init_func: "wizer.initialize".into(),
            allow_wasi: false,
        }
    }

    /// The export name of the initializer function.
    ///
    /// Defaults to `"wizer.initialize"`.
    pub fn init_func(&mut self, init_func: impl Into<String>) -> &mut Self {
        self.init_func = init_func.into();
        self
    }

    /// Allow WASI imports to be called during initialization?
    ///
    /// This can introduce diverging semantics because the initialization can
    /// observe nondeterminism that might have gone a different way at runtime
    /// than it did at initialization time.
    ///
    /// If your Wasm module uses WASI's `get_random` to add randomness to
    /// something as a security mitigation (e.g. something akin to ASLR or the
    /// way Rust's hash maps incorporate a random nonce) then note that, if the
    /// randomization is added during initialization time and you don't ever
    /// re-randomize at runtime, then that randomization will become per-module
    /// rather than per-instance.
    ///
    /// Defaults to `false`.
    pub fn allow_wasi(&mut self, allow: bool) -> &mut Self {
        self.allow_wasi = allow;
        self
    }

    /// Initialize the given Wasm, snapshot it, and return the serialized
    /// snapshot as a new, pre-initialized Wasm module.
    pub fn run(&self, wasm: &[u8]) -> anyhow::Result<Vec<u8>> {
        // Make sure we're given valid Wasm from the get go.
        self.wasm_validate(wasm)?;

        let wasm = self.prepare_input_wasm(wasm)?;
        debug_assert!(
            self.wasm_validate(&wasm).is_ok(),
            "if the Wasm was originally valid, then our preparation step shouldn't invalidate it"
        );

        let store = wasmtime::Store::default();
        let module = wasmtime::Module::new(store.engine(), &wasm)?;
        self.validate_init_func(&module)?;

        let instance = self.initialize(&store, &module)?;
        let diff = self.diff(&instance);
        let initialized_wasm = self.rewrite(&wasm, &diff);

        Ok(initialized_wasm)
    }

    fn wasm_features(&self) -> wasmparser::WasmFeatures {
        wasmparser::WasmFeatures {
            // Proposals that we support.
            multi_memory: true,
            multi_value: true,

            // Proposals that we should add support for.
            reference_types: false,
            module_linking: false,
            simd: false,
            threads: false,
            tail_call: false,
            bulk_memory: false,
            memory64: false,
            exceptions: false,

            // We will never want to enable this.
            deterministic_only: false,
        }
    }

    fn wasm_validate(&self, wasm: &[u8]) -> anyhow::Result<()> {
        log::debug!("Validating input Wasm");
        let mut validator = wasmparser::Validator::new();
        validator.wasm_features(self.wasm_features());
        validator.validate_all(wasm)?;
        Ok(())
    }

    /// Rewrite the input Wasm with our own custom exports for all globals, and
    /// memories. This way we can reflect on their values later on in the diff
    /// phase.
    ///
    /// TODO: will have to also export tables once we support reference types.
    fn prepare_input_wasm(&self, full_wasm: &[u8]) -> anyhow::Result<Vec<u8>> {
        log::debug!("Preparing input Wasm");

        let mut wasm = full_wasm;
        let mut parser = wasmparser::Parser::new(0);
        let mut module = wasm_encoder::Module::new();

        // Count how many globals and memories we see in this module, so that we
        // can export them all.
        let mut memory_count = 0;
        let mut global_count = 0;

        loop {
            let (payload, consumed) =
                match parser.parse(wasm, true).context("failed to parse Wasm")? {
                    wasmparser::Chunk::NeedMoreData(_) => anyhow::bail!("invalid Wasm module"),
                    wasmparser::Chunk::Parsed { payload, consumed } => (payload, consumed),
                };
            wasm = &wasm[consumed..];

            use wasmparser::Payload::*;
            use wasmparser::SectionReader;
            match payload {
                Version { .. } => continue,
                TypeSection(types) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Type as u8,
                        data: &full_wasm[types.range().start..types.range().end],
                    });
                }
                ImportSection(imports) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Import as u8,
                        data: &full_wasm[imports.range().start..imports.range().end],
                    });
                }
                AliasSection(_) | InstanceSection(_) | ModuleSection(_) => {
                    anyhow::bail!("module linking is not supported yet")
                }
                FunctionSection(funcs) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Function as u8,
                        data: &full_wasm[funcs.range().start..funcs.range().end],
                    });
                }
                TableSection(tables) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Table as u8,
                        data: &full_wasm[tables.range().start..tables.range().end],
                    });
                }
                MemorySection(mems) => {
                    memory_count += mems.get_count();
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Memory as u8,
                        data: &full_wasm[mems.range().start..mems.range().end],
                    });
                }
                GlobalSection(globals) => {
                    global_count += globals.get_count();
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Global as u8,
                        data: &full_wasm[globals.range().start..globals.range().end],
                    });
                }
                ExportSection(mut exports) => {
                    let count = exports.get_count();
                    let mut exports_encoder = wasm_encoder::ExportSection::new();
                    for _ in 0..count {
                        let export = exports.read()?;
                        exports_encoder.export(
                            export.field,
                            match export.kind {
                                wasmparser::ExternalKind::Function => {
                                    wasm_encoder::Export::Function(export.index)
                                }
                                wasmparser::ExternalKind::Table => {
                                    wasm_encoder::Export::Table(export.index)
                                }
                                wasmparser::ExternalKind::Memory => {
                                    wasm_encoder::Export::Memory(export.index)
                                }
                                wasmparser::ExternalKind::Global => {
                                    wasm_encoder::Export::Global(export.index)
                                }
                                wasmparser::ExternalKind::Type
                                | wasmparser::ExternalKind::Module
                                | wasmparser::ExternalKind::Instance => {
                                    anyhow::bail!("module linking is not supported yet");
                                }
                                wasmparser::ExternalKind::Event => {
                                    anyhow::bail!("exceptions are not supported yet")
                                }
                            },
                        );
                    }
                    // Export all of the globals and memories under known names
                    // so we can manipulate them later.
                    for i in 0..global_count {
                        let name = format!("__wizer_global_{}", i);
                        exports_encoder.export(&name, wasm_encoder::Export::Global(i));
                    }
                    for i in 0..memory_count {
                        let name = format!("__wizer_memory_{}", i);
                        exports_encoder.export(&name, wasm_encoder::Export::Memory(i));
                    }
                    module.section(&exports_encoder);
                }
                StartSection { func, range: _ } => {
                    module.section(&wasm_encoder::StartSection {
                        function_index: func,
                    });
                }
                ElementSection(elems) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Element as u8,
                        data: &full_wasm[elems.range().start..elems.range().end],
                    });
                }
                DataCountSection { .. } => anyhow::bail!("bulk memory is not supported yet"),
                DataSection(data) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Data as u8,
                        data: &full_wasm[data.range().start..data.range().end],
                    });
                }
                CustomSection {
                    name,
                    data,
                    data_offset: _,
                } => {
                    module.section(&wasm_encoder::CustomSection { name, data });
                }
                CodeSectionStart {
                    range,
                    count: _,
                    size: _,
                } => {
                    let data = &full_wasm[range.start..range.end];
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Code as u8,
                        data,
                    });
                }
                CodeSectionEntry(_) => continue,
                ModuleCodeSectionStart { .. } | ModuleCodeSectionEntry { .. } => {
                    anyhow::bail!("module linking is not supported yet")
                }
                UnknownSection { .. } => anyhow::bail!("unknown section"),
                EventSection(_) => anyhow::bail!("exceptions are not supported yet"),
                End => return Ok(module.finish()),
            }
        }
    }

    /// Check that the module exports an initialization function, and that the
    /// function has the correct type.
    fn validate_init_func(&self, module: &wasmtime::Module) -> anyhow::Result<()> {
        log::debug!("Validating the exported initialization function");
        match module.get_export(&self.init_func) {
            Some(wasmtime::ExternType::Func(func_ty)) => {
                if func_ty.params().len() != 0 || func_ty.results().len() != 0 {
                    anyhow::bail!(
                        "the Wasm module's `{}` function export does not have type `[] -> []`",
                        &self.init_func
                    );
                }
            }
            Some(_) => anyhow::bail!(
                "the Wasm module's `{}` export is not a function",
                &self.init_func
            ),
            None => anyhow::bail!(
                "the Wasm module does not have a `{}` export",
                &self.init_func
            ),
        }
        Ok(())
    }

    /// Create dummy imports for instantiating the module.
    fn dummy_imports(
        &self,
        store: &wasmtime::Store,
        module: &wasmtime::Module,
        linker: &mut wasmtime::Linker,
    ) -> anyhow::Result<()> {
        log::debug!("Creating dummy imports");

        for imp in module.imports() {
            if linker.get_one_by_name(imp.module(), imp.name()).is_ok() {
                // Already defined, must be part of WASI.
                continue;
            }

            match imp.ty() {
                wasmtime::ExternType::Func(func_ty) => {
                    let trap = wasmtime::Trap::new(format!(
                        "cannot call imports within the initialization function; attempted \
                         to call `'{}' '{}'`",
                        imp.module(),
                        imp.name()
                    ));
                    linker.define(
                        imp.module(),
                        imp.name(),
                        wasmtime::Func::new(
                            store,
                            func_ty,
                            move |_caller: wasmtime::Caller, _params, _results| Err(trap.clone()),
                        ),
                    )?;
                }
                wasmtime::ExternType::Global(_global_ty) => {
                    // The Wasm module could use `global.get` to read the
                    // imported value and branch on that or use `global.set` to
                    // update it if it's mutable. We can't create a trapping
                    // dummy value, like we can for functions, and we can't
                    // define a "global segment" to update any imported values
                    // (although I suppose we could inject a `start` function).
                    anyhow::bail!("cannot initialize Wasm modules that import globals")
                }
                wasmtime::ExternType::Table(_table_ty) => {
                    // TODO: we could import a dummy table full of trapping
                    // functions *if* the reference types proposal is not
                    // enabled, so there is no way the Wasm module can
                    // manipulate the table. This would allow initializing such
                    // modules, as long as the initialization didn't do any
                    // `call_indirect`s.
                    anyhow::bail!("cannot initialize Wasm modules that import tables")
                }
                wasmtime::ExternType::Memory(_memory_ty) => {
                    // The Wasm module could read the memory and branch on its
                    // contents, and since we can't create a dummy memory that
                    // matches the "real" import, nor can we create a trapping
                    // dummy version like we can for functions, we can't support
                    // imported memories.
                    anyhow::bail!("cannot initialize Wasm modules that import memories")
                }
            };
        }

        Ok(())
    }

    /// Instantiate the module and call its initialization function.
    fn initialize(
        &self,
        store: &wasmtime::Store,
        module: &wasmtime::Module,
    ) -> anyhow::Result<wasmtime::Instance> {
        log::debug!("Calling the initialization function");

        let mut linker = wasmtime::Linker::new(store);
        if self.allow_wasi {
            let ctx = wasmtime_wasi::WasiCtx::new(None::<String>)?;
            let wasi = wasmtime_wasi::Wasi::new(store, ctx);
            wasi.add_to_linker(&mut linker)?;
        }
        self.dummy_imports(&store, &module, &mut linker)?;
        let instance = linker.instantiate(module)?;

        let init_func = instance
            .get_func(&self.init_func)
            .expect("checked by `validate_init_func`")
            .get0::<()>()
            .expect("checked by `validate_init_func`");
        init_func().with_context(|| format!("the `{}` function trapped", self.init_func))?;

        Ok(instance)
    }

    /// Diff the given instance's globals, memories, and tables from the Wasm
    /// defaults.
    ///
    /// TODO: when we support reference types, we will have to diff tables.
    fn diff<'a>(&self, instance: &'a wasmtime::Instance) -> Diff<'a> {
        // Get the initialized values of all globals.
        log::debug!("Diffing global values");
        let mut globals = vec![];
        let mut global_index = 0;
        loop {
            let name = format!("__wizer_global_{}", global_index);
            match instance.get_global(&name) {
                None => break,
                Some(global) => {
                    globals.push(global.get());
                    global_index += 1;
                }
            }
        }

        // Find and record non-zero regions of memory (in parallel).
        //
        // TODO: This could be really slow for large memories. Instead, we
        // should bring our own memories, protect the pages, and keep a table
        // with a dirty bit for each page, so we can just diff the pages that
        // actually got changed to non-zero values.
        log::debug!("Diffing memories");
        let mut memory_mins = vec![];
        let mut data_segments = vec![];
        let mut memory_index = 0;
        loop {
            let name = format!("__wizer_memory_{}", memory_index);
            match instance.get_memory(&name) {
                None => break,
                Some(memory) => {
                    memory_mins.push(memory.size());

                    let num_wasm_pages = memory.size();
                    let num_native_pages = num_wasm_pages * (WASM_PAGE_SIZE / NATIVE_PAGE_SIZE);

                    let memory: &'a [u8] = unsafe {
                        // Safe because no one else has a (potentially mutable)
                        // view to this memory and we know the memory will live
                        // as long as the instance is alive.
                        std::slice::from_raw_parts(memory.data_ptr(), memory.data_size())
                    };

                    // Consider each "native" page of the memory. (Scare quotes
                    // because we have no guarantee that anyone isn't using huge
                    // page sizes or something). Process each page in
                    // parallel. If any byte has changed, add the whole page as
                    // a data segment. This means that the resulting Wasm module
                    // should instantiate faster, since there are fewer segments
                    // to bounds check on instantiation. Engines could even
                    // theoretically recognize that each of these segments is
                    // page sized and aligned, and use lazy copy-on-write
                    // initialization of each instance's memory.
                    data_segments.par_extend((0..num_native_pages).into_par_iter().filter_map(
                        |i| {
                            let start = i * NATIVE_PAGE_SIZE;
                            let end = ((i + 1) * NATIVE_PAGE_SIZE) as usize;
                            let page = &memory[start as usize..end];
                            for byte in page {
                                if *byte != 0 {
                                    return Some(DataSegment {
                                        memory_index,
                                        offset: start as u32,
                                        data: page,
                                    });
                                }
                            }
                            None
                        },
                    ));

                    memory_index += 1;
                }
            }
        }

        // Sort data segments to enforce determinism in the face of the
        // parallelism above.
        data_segments.sort_by_key(|s| (s.memory_index, s.offset));

        // Merge any contiguous pages, so that the engine can initialize them
        // all at once (ideally with a single copy-on-write `mmap`) rather than
        // initializing each data segment individually.
        for i in (1..data_segments.len()).rev() {
            let a = &data_segments[i - 1];
            let b = &data_segments[i];

            // Only merge segments for the same memory.
            if a.memory_index != b.memory_index {
                continue;
            }

            // Only merge segments if they are contiguous.
            if a.offset + u32::try_from(a.data.len()).unwrap() != b.offset {
                continue;
            }

            // Okay, merge them together into `a` (so that the next iteration
            // can merge it with its predecessor) and then remove `b`!
            data_segments[i - 1].data = unsafe {
                debug_assert_eq!(
                    a.data
                        .as_ptr()
                        .offset(isize::try_from(a.data.len()).unwrap()),
                    b.data.as_ptr()
                );
                std::slice::from_raw_parts(a.data.as_ptr(), a.data.len() + b.data.len())
            };
            data_segments.remove(i);
        }

        Diff {
            globals,
            memory_mins,
            data_segments,
        }
    }

    fn rewrite(&self, full_wasm: &[u8], diff: &Diff) -> Vec<u8> {
        log::debug!("Rewriting input Wasm to pre-initialized state");

        let mut wasm = full_wasm;
        let mut parser = wasmparser::Parser::new(0);
        let mut module = wasm_encoder::Module::new();

        // Encode the initialized data segments from the diff rather
        // than the original, uninitialized data segments.
        let mut added_data = false;
        let mut add_data_section = |module: &mut wasm_encoder::Module| {
            if added_data || diff.data_segments.is_empty() {
                return;
            }
            let mut data_section = wasm_encoder::DataSection::new();
            for DataSegment {
                memory_index,
                offset,
                data,
            } in &diff.data_segments
            {
                data_section.active(
                    *memory_index,
                    wasm_encoder::Instruction::I32Const(*offset as i32),
                    data.iter().copied(),
                );
            }
            module.section(&data_section);
            added_data = true;
        };

        loop {
            let (payload, consumed) = match parser.parse(wasm, true).unwrap() {
                wasmparser::Chunk::NeedMoreData(_) => unreachable!(),
                wasmparser::Chunk::Parsed { payload, consumed } => (payload, consumed),
            };
            wasm = &wasm[consumed..];

            use wasmparser::Payload::*;
            use wasmparser::SectionReader;
            match payload {
                Version { .. } => continue,
                TypeSection(types) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Type as u8,
                        data: &full_wasm[types.range().start..types.range().end],
                    });
                }
                ImportSection(imports) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Import as u8,
                        data: &full_wasm[imports.range().start..imports.range().end],
                    });
                }
                AliasSection(_) | InstanceSection(_) | ModuleSection(_) => {
                    unreachable!()
                }
                FunctionSection(funcs) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Function as u8,
                        data: &full_wasm[funcs.range().start..funcs.range().end],
                    });
                }
                TableSection(tables) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Table as u8,
                        data: &full_wasm[tables.range().start..tables.range().end],
                    });
                }
                MemorySection(mut mems) => {
                    // Set the minimum size of each memory to the diff's
                    // initialized size for that memory.
                    let mut memory_encoder = wasm_encoder::MemorySection::new();
                    for i in 0..mems.get_count() {
                        let memory = mems.read().unwrap();
                        match memory {
                            wasmparser::MemoryType::M32 { limits, shared: _ } => {
                                memory_encoder.memory(wasm_encoder::MemoryType {
                                    limits: wasm_encoder::Limits {
                                        min: diff.memory_mins[i as usize],
                                        max: limits.maximum,
                                    },
                                });
                            }
                            _ => unreachable!(),
                        }
                    }
                    module.section(&memory_encoder);
                }
                GlobalSection(mut globals) => {
                    // Encode the initialized values from the diff, rather than
                    // the original values.
                    let mut globals_encoder = wasm_encoder::GlobalSection::new();
                    for i in 0..globals.get_count() {
                        let global = globals.read().unwrap();
                        globals_encoder.global(
                            translate_global_type(global.ty),
                            match diff.globals[i as usize] {
                                wasmtime::Val::I32(x) => wasm_encoder::Instruction::I32Const(x),
                                wasmtime::Val::I64(x) => wasm_encoder::Instruction::I64Const(x),
                                wasmtime::Val::F32(x) => {
                                    wasm_encoder::Instruction::F32Const(f32::from_bits(x))
                                }
                                wasmtime::Val::F64(x) => {
                                    wasm_encoder::Instruction::F64Const(f64::from_bits(x))
                                }
                                _ => unreachable!(),
                            },
                        );
                    }
                    module.section(&globals_encoder);
                }
                ExportSection(mut exports) => {
                    // Remove the `__wizer_*` exports we added during the
                    // preparation phase, as well as the initialization
                    // function's export. Removing the latter will enable
                    // further Wasm optimizations (notably GC'ing unused
                    // functions) via `wasm-opt` and similar tools.
                    let count = exports.get_count();
                    let mut exports_encoder = wasm_encoder::ExportSection::new();
                    for _ in 0..count {
                        let export = exports.read().unwrap();
                        if export.field.starts_with("__wizer_") || export.field == self.init_func {
                            continue;
                        }
                        exports_encoder.export(
                            export.field,
                            match export.kind {
                                wasmparser::ExternalKind::Function => {
                                    wasm_encoder::Export::Function(export.index)
                                }
                                wasmparser::ExternalKind::Table => {
                                    wasm_encoder::Export::Table(export.index)
                                }
                                wasmparser::ExternalKind::Memory => {
                                    wasm_encoder::Export::Memory(export.index)
                                }
                                wasmparser::ExternalKind::Global => {
                                    wasm_encoder::Export::Global(export.index)
                                }
                                wasmparser::ExternalKind::Type
                                | wasmparser::ExternalKind::Module
                                | wasmparser::ExternalKind::Instance
                                | wasmparser::ExternalKind::Event => {
                                    unreachable!()
                                }
                            },
                        );
                    }
                    module.section(&exports_encoder);
                }
                StartSection { .. } => {
                    // Skip the `start` function -- it's already been run!
                    continue;
                }
                ElementSection(elems) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Element as u8,
                        data: &full_wasm[elems.range().start..elems.range().end],
                    });
                }
                DataCountSection { .. } => unreachable!(),
                DataSection(_) => {
                    // TODO: supporting bulk memory will require copying over
                    // any active or declared segments.
                    add_data_section(&mut module);
                }
                CustomSection {
                    name,
                    data,
                    data_offset: _,
                } => {
                    // Some tools expect the name custom section to come last,
                    // even though custom sections are allowed in any order.
                    if name == "name" {
                        add_data_section(&mut module);
                    }

                    module.section(&wasm_encoder::CustomSection { name, data });
                }
                CodeSectionStart {
                    range,
                    count: _,
                    size: _,
                } => {
                    let data = &full_wasm[range.start..range.end];
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Code as u8,
                        data,
                    });
                }
                CodeSectionEntry(_) => continue,
                ModuleCodeSectionStart { .. }
                | ModuleCodeSectionEntry { .. }
                | UnknownSection { .. }
                | EventSection(_) => unreachable!(),
                End => {
                    add_data_section(&mut module);
                    return module.finish();
                }
            }
        }
    }
}

/// A "diff" of Wasm state from its default value after having been initialized.
struct Diff<'a> {
    /// Maps global index to its initialized value.
    globals: Vec<wasmtime::Val>,

    /// A new minimum size for each memory (in units of pages).
    memory_mins: Vec<u32>,

    /// Segments of non-zero memory.
    data_segments: Vec<DataSegment<'a>>,
}

struct DataSegment<'a> {
    /// The index of this data segment's memory.
    memory_index: u32,
    /// The offset within the memory that `data` should be copied to.
    offset: u32,
    /// This segment's (non-zero) data.
    data: &'a [u8],
}
