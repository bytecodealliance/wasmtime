//! TODO FITZGEN

#![deny(missing_docs)]

use anyhow::Context;
use std::convert::TryFrom;
use structopt::StructOpt;

/// Wizer: the WebAssembly initializer.
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
    #[structopt(short = "f", long = "init-func", default_value = "wizer.initialize")]
    init_func: String,
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

fn translate_limits(limits: wasmparser::ResizableLimits) -> wasm_encoder::Limits {
    wasm_encoder::Limits {
        min: limits.initial,
        max: limits.maximum,
    }
}

fn translate_table_type(ty: wasmparser::TableType) -> anyhow::Result<wasm_encoder::TableType> {
    anyhow::ensure!(
        ty.element_type == wasmparser::Type::FuncRef,
        "only funcref tables are supported"
    );
    Ok(wasm_encoder::TableType {
        limits: translate_limits(ty.limits),
    })
}

fn translate_memory_type(ty: wasmparser::MemoryType) -> anyhow::Result<wasm_encoder::MemoryType> {
    match ty {
        wasmparser::MemoryType::M32 { limits, shared } => {
            anyhow::ensure!(!shared, "shared memories are not supported yet");
            Ok(wasm_encoder::MemoryType {
                limits: translate_limits(limits),
            })
        }
        wasmparser::MemoryType::M64 { .. } => {
            anyhow::bail!("64-bit memories not supported yet")
        }
    }
}

fn translate_global_type(ty: wasmparser::GlobalType) -> wasm_encoder::GlobalType {
    wasm_encoder::GlobalType {
        val_type: translate_val_type(ty.content_type),
        mutable: ty.mutable,
    }
}

fn translate_init_expr(expr: wasmparser::InitExpr) -> anyhow::Result<wasm_encoder::Instruction> {
    let mut ops = expr.get_operators_reader();
    let init = match ops.read()? {
        wasmparser::Operator::GlobalGet { global_index } => {
            wasm_encoder::Instruction::GlobalGet(global_index)
        }
        wasmparser::Operator::I32Const { value } => wasm_encoder::Instruction::I32Const(value),
        wasmparser::Operator::I64Const { value } => wasm_encoder::Instruction::I64Const(value),
        wasmparser::Operator::F32Const { value } => {
            wasm_encoder::Instruction::F32Const(f32::from_bits(value.bits()))
        }
        wasmparser::Operator::F64Const { value } => {
            wasm_encoder::Instruction::F64Const(f64::from_bits(value.bits()))
        }
        _ => anyhow::bail!("unsupported init expr"),
    };
    anyhow::ensure!(
        matches!(ops.read()?, wasmparser::Operator::End),
        "expected `end` instruction"
    );
    ops.ensure_end()?;
    Ok(init)
}

impl Wizer {
    /// Construct a new `Wizer` builder.
    pub fn new() -> Self {
        Wizer {
            init_func: "wizer.initialize".into(),
        }
    }

    /// The export name of the initializer function.
    ///
    /// Defaults to `"wizer.initialize"`.
    pub fn init_func(&mut self, init_func: impl Into<String>) -> &mut Self {
        self.init_func = init_func.into();
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
        let imports = self.dummy_imports(&store, &module)?;
        let instance = wasmtime::Instance::new(&store, &module, &imports)?;

        self.initialize(&instance)?;
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
            match payload {
                Version { .. } => continue,
                TypeSection(mut types) => {
                    let count = types.get_count();
                    let mut types_encoder = wasm_encoder::TypeSection::new();
                    for _ in 0..count {
                        match types.read()? {
                            wasmparser::TypeDef::Func(ft) => {
                                types_encoder.function(
                                    ft.params.iter().copied().map(translate_val_type),
                                    ft.returns.iter().copied().map(translate_val_type),
                                );
                            }
                            wasmparser::TypeDef::Instance(_) | wasmparser::TypeDef::Module(_) => {
                                anyhow::bail!("module linking is not supported yet");
                            }
                        }
                    }
                    module.section(&types_encoder);
                }
                ImportSection(mut imports) => {
                    let count = imports.get_count();
                    let mut imports_encoder = wasm_encoder::ImportSection::new();
                    for _ in 0..count {
                        let imp = imports.read()?;
                        imports_encoder.import(
                            imp.module,
                            imp.field.expect(
                                "should always be `Some` when module linking isn't enabled",
                            ),
                            match imp.ty {
                                wasmparser::ImportSectionEntryType::Function(ty) => {
                                    wasm_encoder::ImportType::Function(ty)
                                }
                                wasmparser::ImportSectionEntryType::Table(ty) => {
                                    translate_table_type(ty)?.into()
                                }
                                wasmparser::ImportSectionEntryType::Memory(ty) => {
                                    memory_count += 1;
                                    translate_memory_type(ty)?.into()
                                }
                                wasmparser::ImportSectionEntryType::Global(ty) => {
                                    global_count += 1;
                                    translate_global_type(ty).into()
                                }
                                wasmparser::ImportSectionEntryType::Module(_)
                                | wasmparser::ImportSectionEntryType::Instance(_) => {
                                    anyhow::bail!("module linking is not supported yet")
                                }
                                wasmparser::ImportSectionEntryType::Event(_) => {
                                    anyhow::bail!("exceptions are not supported yet")
                                }
                            },
                        );
                    }
                    module.section(&imports_encoder);
                }
                AliasSection(_) | InstanceSection(_) | ModuleSection(_) => {
                    anyhow::bail!("module linking is not supported yet")
                }
                FunctionSection(mut funcs) => {
                    let count = funcs.get_count();
                    let mut funcs_encoder = wasm_encoder::FunctionSection::new();
                    for _ in 0..count {
                        let ty_idx = funcs.read()?;
                        funcs_encoder.function(ty_idx);
                    }
                    module.section(&funcs_encoder);
                }
                TableSection(mut tables) => {
                    let count = tables.get_count();
                    let mut tables_encoder = wasm_encoder::TableSection::new();
                    for _ in 0..count {
                        let table_ty = tables.read()?;
                        tables_encoder.table(translate_table_type(table_ty)?);
                    }
                    module.section(&tables_encoder);
                }
                MemorySection(mut mems) => {
                    let count = mems.get_count();
                    memory_count += count;
                    let mut mems_encoder = wasm_encoder::MemorySection::new();
                    for _ in 0..count {
                        let mem_ty = mems.read()?;
                        mems_encoder.memory(translate_memory_type(mem_ty)?);
                    }
                    module.section(&mems_encoder);
                }
                GlobalSection(mut globals) => {
                    let count = globals.get_count();
                    global_count += count;
                    let mut globals_encoder = wasm_encoder::GlobalSection::new();
                    for _ in 0..count {
                        let global = globals.read()?;
                        globals_encoder.global(
                            translate_global_type(global.ty),
                            translate_init_expr(global.init_expr)?,
                        );
                    }
                    module.section(&globals_encoder);
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
                ElementSection(mut elems) => {
                    let count = elems.get_count();
                    let mut elems_encoder = wasm_encoder::ElementSection::new();
                    for _ in 0..count {
                        let elem = elems.read()?;
                        match elem.kind {
                            wasmparser::ElementKind::Active {
                                table_index,
                                init_expr,
                            } => {
                                let init_expr = translate_init_expr(init_expr)?;
                                let mut items = elem.items.get_items_reader()?;
                                let mut funcs = Vec::with_capacity(items.get_count() as usize);
                                for _ in 0..items.get_count() {
                                    funcs.push(match items.read()? {
                                        wasmparser::ElementItem::Func(idx) => idx,
                                        wasmparser::ElementItem::Null(_) => {
                                            anyhow::bail!("reference types are not supported yet")
                                        }
                                    });
                                }
                                elems_encoder.active(table_index, init_expr, funcs.drain(..));
                            }
                            wasmparser::ElementKind::Passive
                            | wasmparser::ElementKind::Declared => {
                                anyhow::bail!("bulk memory is not supported yet")
                            }
                        }
                    }
                    module.section(&elems_encoder);
                }
                DataCountSection { .. } => anyhow::bail!("bulk memory is not supported yet"),
                DataSection(mut data) => {
                    let count = data.get_count();
                    let mut data_encoder = wasm_encoder::DataSection::new();
                    for _ in 0..count {
                        let segment = data.read()?;
                        match segment.kind {
                            wasmparser::DataKind::Active {
                                memory_index,
                                init_expr,
                            } => {
                                let init_expr = translate_init_expr(init_expr)?;
                                data_encoder.active(
                                    memory_index,
                                    init_expr,
                                    segment.data.iter().copied(),
                                );
                            }
                            wasmparser::DataKind::Passive => {
                                anyhow::bail!("bulk memory is not supported yet")
                            }
                        }
                    }
                    module.section(&data_encoder);
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
    ) -> anyhow::Result<Vec<wasmtime::Extern>> {
        log::debug!("Creating dummy imports");

        let mut imports = Vec::with_capacity(module.imports().len());
        for imp in module.imports() {
            imports.push(match imp.ty() {
                wasmtime::ExternType::Func(func_ty) => {
                    let trap = wasmtime::Trap::new(format!(
                        "cannot call imports within the initialization function; attempted \
                         to call `'{}' '{}'`",
                        imp.module(),
                        imp.name()
                    ));
                    wasmtime::Func::new(store, func_ty, move |_caller, _params, _results| {
                        Err(trap.clone())
                    })
                    .into()
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
                wasmtime::ExternType::Module(_) | wasmtime::ExternType::Instance(_) => {
                    anyhow::bail!("module linking is not supported yet")
                }
            });
        }
        Ok(imports)
    }

    /// Call the initialization function.
    fn initialize(&self, instance: &wasmtime::Instance) -> anyhow::Result<()> {
        log::debug!("Calling the initialization function");
        let init_func = instance
            .get_func(&self.init_func)
            .expect("checked by `validate_init_func`")
            .get0::<()>()
            .expect("checked by `validate_init_func`");
        init_func().with_context(|| format!("the `{}` function trapped", self.init_func))?;
        Ok(())
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

        // Find and record non-zero regions of memory.
        //
        // TODO: This could be really slow for large memories. Instead, we
        // should bring our own memories, protect the pages, and keep a table
        // with a dirty bit for each page, so we can just diff the pages that
        // actually got changed to non-zero values.
        log::debug!("Diffing memories");
        let mut data_segments = vec![];
        let mut memory_index = 0;
        loop {
            let name = format!("__wizer_memory_{}", memory_index);
            match instance.get_memory(&name) {
                None => break,
                Some(memory) => {
                    let memory: &'a [u8] = unsafe {
                        // Safe because no one else has a (potentially mutable)
                        // view to this memory and we know the memory will live
                        // as long as the instance is alive.
                        std::slice::from_raw_parts(memory.data_ptr(), memory.data_size())
                    };

                    let mut i = 0;
                    loop {
                        // Search for the start of a non-zero region of
                        // memory. After the loop `i` will either be out of
                        // bounds, or be the start of the non-zero region.
                        while i < memory.len() && memory[i] == 0 {
                            i += 1;
                            continue;
                        }

                        if i >= memory.len() {
                            break;
                        }

                        // We found the start of a non-zero region, now
                        // search for its end. `j` will be the end of the
                        // non-zero region.
                        let mut j = i + 1;
                        while j < memory.len() && memory[j] != 0 {
                            j += 1;
                        }

                        // Remember this non-zero region as a data segment
                        // for the pre-initialized module.
                        debug_assert!(memory[i..j].iter().all(|b| *b != 0));
                        data_segments.push((
                            memory_index,
                            u32::try_from(i).unwrap(),
                            &memory[i..j],
                        ));

                        // Continue the search for the start of a non-zero
                        // region from the end of this non-zero region.
                        i = j + 1;
                    }

                    memory_index += 1;
                }
            }
        }

        Diff {
            globals,
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
            for &(memory_index, offset, data) in &diff.data_segments {
                data_section.active(
                    memory_index,
                    wasm_encoder::Instruction::I32Const(offset as i32),
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
                MemorySection(mems) => {
                    module.section(&wasm_encoder::RawSection {
                        id: wasm_encoder::SectionId::Memory as u8,
                        data: &full_wasm[mems.range().start..mems.range().end],
                    });
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
                    // preparation phase.
                    let count = exports.get_count();
                    let mut exports_encoder = wasm_encoder::ExportSection::new();
                    for _ in 0..count {
                        let export = exports.read().unwrap();
                        if export.field.starts_with("__wizer_") {
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

    /// Segments of non-zero memory.
    ///
    /// `(memory_index, offset, data)`.
    data_segments: Vec<(u32, u32, &'a [u8])>,
}
