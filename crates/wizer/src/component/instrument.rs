use crate::ModuleContext;
use crate::component::info::{Accessor, RawSection};
use crate::component::{ComponentContext, WIZER_INSTANCE};
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
use wasmtime::{Result, bail};

/// Instrumentation phase of wizening a component.
///
/// This is similar to the core wasm wizening instrumentation but operates at
/// the component level. This notably handles multiple instances and multiple
/// globals/memories across these instances. The general idea of the
/// instrumented component is:
///
/// * All core modules are instrumented with typical wizer instrumentation (e.g.
///   `__wizer_*` exports of all internal state).
/// * A core module is then generated which accesses all of these exports in the
///   form of functions.
/// * This core module is instantiated and lifted in a component instance which
///   contains exported functions for accessing all of the various pieces of
///   state of the module.
/// * The lifted instance is exported under the `WIZER_INSTANCE` name.
///
/// The main goal is to reuse the core wasm instrumentation as much as possible
/// here, and then the only remaining question is how to plumb all the core wasm
/// state out of the component through WIT.
pub(crate) fn instrument(component: &mut ComponentContext<'_>) -> Result<Vec<u8>> {
    let mut encoder = wasm_encoder::Component::new();

    // First pass through all sections as-is, ensuring that module sections are
    // the instrumented version of the module.
    for section in component.sections.iter_mut() {
        match section {
            RawSection::Raw(raw) => {
                encoder.section(raw);
            }
            RawSection::Module(module) => {
                let wasm = crate::instrument::instrument(module);
                encoder.section(&wasm_encoder::RawSection {
                    id: wasm_encoder::ComponentSectionId::CoreModule as u8,
                    data: &wasm,
                });
            }
        }
    }

    // Build the accessor module and append it with this helper.
    let mut builder = AccessorBuilder {
        component,
        accessor_types: Default::default(),
        accessor_imports: Default::default(),
        accessor_functions: Default::default(),
        accessor_code: Default::default(),
        accessor_exports: Default::default(),
        accessor_nglobals: 0,
        accessor_nmemories: 0,
        instances_to_instantiate_with: Vec::new(),
        accessors: Vec::new(),
        extra_types: Default::default(),
        extra_aliases: Default::default(),
        extra_canonicals: Default::default(),
        accessor_instance_export_items: Vec::new(),
        extra_core_funcs: 0,
    };
    builder.build(&mut encoder)?;
    component.accessors = Some(builder.accessors);

    Ok(encoder.finish())
}

struct AccessorBuilder<'a> {
    component: &'a ComponentContext<'a>,

    // Sections that are used to create the "accessor" module which is a bunch
    // of functions that reads the internal state of all other instances in this
    // component.
    accessor_types: wasm_encoder::TypeSection,
    accessor_imports: wasm_encoder::ImportSection,
    accessor_functions: wasm_encoder::FunctionSection,
    accessor_code: wasm_encoder::CodeSection,
    accessor_exports: wasm_encoder::ExportSection,
    accessor_nglobals: u32,
    accessor_nmemories: u32,

    // Arguments to the instantiation of the accessor module.
    instances_to_instantiate_with: Vec<(u32, String)>,

    // All accessor functions generated for all instance internal state.
    accessors: Vec<Accessor>,

    // Sections that are appended to the component as part of the
    // instrumentation. This is the implementation detail of lifting all the
    // functions in the "accessor" module.
    extra_types: wasm_encoder::ComponentTypeSection,
    extra_aliases: wasm_encoder::ComponentAliasSection,
    extra_core_funcs: u32,
    extra_canonicals: wasm_encoder::CanonicalFunctionSection,
    accessor_instance_export_items: Vec<(String, wasm_encoder::ComponentExportKind, u32)>,
}

impl AccessorBuilder<'_> {
    fn build(&mut self, encoder: &mut wasm_encoder::Component) -> Result<()> {
        for (module_index, module) in self.component.core_modules() {
            let instance_index = match self.component.core_instantiations.get(&module_index) {
                Some(i) => *i,
                None => continue,
            };
            self.add_core_instance(module_index, module, instance_index)?;
        }

        self.finish(encoder);
        Ok(())
    }

    fn add_core_instance(
        &mut self,
        module_index: u32,
        module: &ModuleContext<'_>,
        instance_index: u32,
    ) -> Result<()> {
        let instance_import_name = instance_index.to_string();

        for (_, ty, name) in module.defined_globals() {
            let name = match name {
                Some(n) => n,
                None => continue,
            };

            let accessor_export_name =
                self.add_core_instance_global(&instance_import_name, name, ty)?;
            self.accessors.push(Accessor::Global {
                module_index,
                accessor_export_name,
                ty: ty.content_type,
                core_export_name: name.to_string(),
            });
            self.accessor_nglobals += 1;
        }

        let defined_memory_exports = module.defined_memory_exports.as_ref().unwrap();
        for ((_, ty), name) in module.defined_memories().zip(defined_memory_exports) {
            let accessor_export_name =
                self.add_core_instance_memory(instance_index, &instance_import_name, name, ty);
            self.accessors.push(Accessor::Memory {
                module_index,
                accessor_export_name,
                core_export_name: name.to_string(),
            });
            self.accessor_nmemories += 1;
        }

        self.instances_to_instantiate_with
            .push((instance_index, instance_import_name));

        Ok(())
    }

    fn add_core_instance_global(
        &mut self,
        instance_import_name: &str,
        global_export_name: &str,
        global_ty: wasmparser::GlobalType,
    ) -> Result<String> {
        // Import the global and then define a function which returns the
        // current value of the global.
        self.accessor_imports.import(
            &instance_import_name,
            global_export_name,
            RoundtripReencoder.global_type(global_ty).unwrap(),
        );
        let type_index = self.accessor_types.len();
        self.accessor_types.ty().function(
            [],
            [RoundtripReencoder.val_type(global_ty.content_type).unwrap()],
        );
        self.accessor_functions.function(type_index);

        // Accessing a global is pretty easy, it's just `global.get`
        let mut function = wasm_encoder::Function::new([]);
        let mut ins = function.instructions();
        ins.global_get(self.accessor_nglobals);
        ins.end();
        self.accessor_code.function(&function);

        let prim = match global_ty.content_type {
            wasmparser::ValType::I32 => wasm_encoder::PrimitiveValType::S32,
            wasmparser::ValType::I64 => wasm_encoder::PrimitiveValType::S64,
            wasmparser::ValType::F32 => wasm_encoder::PrimitiveValType::F32,
            wasmparser::ValType::F64 => wasm_encoder::PrimitiveValType::F64,
            wasmparser::ValType::V128 => bail!("component wizening does not support v128 globals"),
            wasmparser::ValType::Ref(_) => unreachable!(),
        };
        let lift_type_index = self.component.types + self.extra_types.len();
        self.extra_types
            .function()
            .params::<_, wasm_encoder::ComponentValType>([])
            .result(Some(wasm_encoder::ComponentValType::Primitive(prim)));
        Ok(self.lift_accessor("global", lift_type_index, &[]))
    }

    fn add_core_instance_memory(
        &mut self,
        instance_index: u32,
        instance_import_name: &str,
        memory_export_name: &str,
        memory_ty: wasmparser::MemoryType,
    ) -> String {
        self.accessor_imports.import(
            &instance_import_name,
            memory_export_name,
            RoundtripReencoder.memory_type(memory_ty).unwrap(),
        );
        let type_index = self.accessor_types.len();
        self.accessor_types
            .ty()
            .function([], [wasm_encoder::ValType::I32]);
        self.accessor_functions.function(type_index);

        // Accessing a linear memory is more subtle than a global. We're
        // returning a `list<u8>` in WIT but to do so we have to store the
        // ptr/length in memory itself. To work around this the memory is grown
        // by a single page to ensure we don't tamper with the original image,
        // and then in this new page the ptr/len are stored. The base pointer is
        // always 0 and the length is the size of memory prior to the growth.
        let mut function = wasm_encoder::Function::new([(1, wasm_encoder::ValType::I32)]);
        let mut ins = function.instructions();

        // Grow memory by 1 page, and trap if the growth failed.
        let pages_to_grow_by = match memory_ty.page_size_log2 {
            Some(0) => 8,
            Some(16) | None => 1,
            _ => unreachable!(),
        };
        ins.i32_const(pages_to_grow_by);
        ins.memory_grow(self.accessor_nmemories);
        ins.local_tee(0);
        ins.i32_const(-1);
        ins.i32_eq();
        ins.if_(wasm_encoder::BlockType::Empty);
        ins.unreachable();
        ins.end();

        // Update our one local as the full byte length of memory.
        ins.local_get(0);
        ins.i32_const(memory_ty.page_size_log2.unwrap_or(16).cast_signed());
        ins.i32_shl();
        ins.local_set(0);

        let memarg = |offset| wasm_encoder::MemArg {
            align: 2,
            offset,
            memory_index: self.accessor_nmemories,
        };
        // Store the ptr/len into the page that was just allocated
        ins.local_get(0);
        ins.i32_const(0);
        ins.i32_store(memarg(0));
        ins.local_get(0);
        ins.local_get(0);
        ins.i32_store(memarg(4));

        // and return the local as it's the pointer to the ptr/len combo
        ins.local_get(0);

        ins.end();
        self.accessor_code.function(&function);

        let list_ty = self.component.types + self.extra_types.len();
        self.extra_types
            .defined_type()
            .list(wasm_encoder::ComponentValType::Primitive(
                wasm_encoder::PrimitiveValType::U8,
            ));
        let lift_type_index = self.component.types + self.extra_types.len();
        self.extra_types
            .function()
            .params::<_, wasm_encoder::ComponentValType>([])
            .result(Some(wasm_encoder::ComponentValType::Type(list_ty)));
        self.extra_aliases
            .alias(wasm_encoder::Alias::CoreInstanceExport {
                instance: instance_index,
                kind: wasm_encoder::ExportKind::Memory,
                name: memory_export_name,
            });
        self.lift_accessor(
            "memory",
            lift_type_index,
            &[wasm_encoder::CanonicalOption::Memory(
                self.component.core_memories + self.accessor_nmemories,
            )],
        )
    }

    fn lift_accessor(
        &mut self,
        item: &str,
        lift_type_index: u32,
        opts: &[wasm_encoder::CanonicalOption],
    ) -> String {
        let accessor_core_export_name = self.accessors.len().to_string();
        self.accessor_exports.export(
            &accessor_core_export_name,
            wasm_encoder::ExportKind::Func,
            self.accessor_functions.len() - 1,
        );

        self.extra_aliases
            .alias(wasm_encoder::Alias::CoreInstanceExport {
                instance: self.accessor_instance_index(),
                kind: wasm_encoder::ExportKind::Func,
                name: &accessor_core_export_name,
            });
        self.extra_core_funcs += 1;
        self.extra_canonicals.lift(
            self.component.core_funcs + self.extra_core_funcs - 1,
            lift_type_index,
            opts.iter().copied(),
        );

        let accessor_export_name = format!("{item}{}", self.accessors.len());
        self.accessor_instance_export_items.push((
            accessor_export_name.clone(),
            wasm_encoder::ComponentExportKind::Func,
            self.component.funcs + self.extra_canonicals.len() - 1,
        ));
        accessor_export_name
    }

    fn finish(&mut self, encoder: &mut wasm_encoder::Component) {
        // Build the `accessor_module` and add it to the component.
        let mut accessor_module = wasm_encoder::Module::new();
        accessor_module.section(&self.accessor_types);
        accessor_module.section(&self.accessor_imports);
        accessor_module.section(&self.accessor_functions);
        accessor_module.section(&self.accessor_exports);
        accessor_module.section(&self.accessor_code);
        encoder.section(&wasm_encoder::ModuleSection(&accessor_module));

        // Instantiate the `accessor_module` with prior instantiations.
        let mut extra_instances = wasm_encoder::InstanceSection::new();
        extra_instances.instantiate(
            self.component.num_core_modules(),
            self.instances_to_instantiate_with
                .iter()
                .map(|(i, name)| (name.as_str(), wasm_encoder::ModuleArg::Instance(*i))),
        );
        encoder.section(&extra_instances);

        // Add instrumentation to the component which extracts names from the
        // accessor instance, lifts things into the component model, and then
        // export them.
        encoder.section(&self.extra_aliases);
        encoder.section(&self.extra_types);
        encoder.section(&self.extra_canonicals);
        let mut extra_component_instances = wasm_encoder::ComponentInstanceSection::new();
        extra_component_instances.export_items(
            self.accessor_instance_export_items
                .iter()
                .map(|(a, b, c)| (a.as_str(), *b, *c)),
        );
        encoder.section(&extra_component_instances);

        let mut extra_exports = wasm_encoder::ComponentExportSection::new();
        extra_exports.export(
            WIZER_INSTANCE,
            wasm_encoder::ComponentExportKind::Instance,
            self.component.instances,
            None,
        );
        encoder.section(&extra_exports);
    }

    fn accessor_instance_index(&self) -> u32 {
        self.component.core_instances
    }
}
