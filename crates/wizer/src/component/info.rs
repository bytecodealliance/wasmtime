use crate::ModuleContext;
use std::collections::HashMap;

/// Wizer-specific contextual information about a component, returned from
/// [`Wizer::instrument_component`].
///
/// [`Wizer::instrument_component`]: crate::Wizer::instrument_component
#[derive(Default)]
pub struct ComponentContext<'a> {
    /// Sections of the component, which are either raw bytes or a parsed module
    /// using `ModuleContext`.
    pub(crate) sections: Vec<RawSection<'a>>,

    /// Counts of each index space for what this component contains.
    ///
    /// Note that these aren't all index spaces in the component, only those
    /// needed at this time.
    pub(crate) instances: u32,
    pub(crate) funcs: u32,
    pub(crate) types: u32,
    pub(crate) core_instances: u32,
    pub(crate) core_memories: u32,
    pub(crate) core_funcs: u32,

    /// Map of which module index to the core instance index it's instantiated
    /// as.
    pub(crate) core_instantiations: HashMap<u32, u32>,

    /// Instrumentation injected to access internal state of globals/memories.
    pub(crate) accessors: Option<Vec<Accessor>>,
}

/// Generated accessors during instrumentation and the metadata about them.
pub(crate) enum Accessor {
    /// This accessor retrieves the value of a wasm global.
    Global {
        /// The module index, within the parent component, that this global
        /// belongs to.
        module_index: u32,

        /// The wizer-instrumented name of the global export this is accessing.
        core_export_name: String,

        /// The component level export name to access this global.
        accessor_export_name: String,

        /// The content type of this global.
        ty: wasmparser::ValType,
    },

    /// This accessor retrieves the value of a wasm linear memory as a
    /// `list<u8>` in WIT.
    Memory {
        /// The module index, within the parent component, that this memory
        /// belongs to.
        module_index: u32,

        /// The wizer-instrumented name of the memory export this is accessing.
        core_export_name: String,

        /// The component level export name to access this memory.
        accessor_export_name: String,
    },
}

/// A section of a component, learned during parsing.
pub(crate) enum RawSection<'a> {
    /// A non-module section, whose raw contents are stored here.
    Raw(wasm_encoder::RawSection<'a>),

    /// A module section, parsed as with Wizer's metadata.
    Module(ModuleContext<'a>),
}

impl<'a> ComponentContext<'a> {
    pub(crate) fn push_raw_section(&mut self, section: wasm_encoder::RawSection<'a>) {
        self.sections.push(RawSection::Raw(section));
    }

    pub(crate) fn push_module_section(&mut self, module: ModuleContext<'a>) {
        self.sections.push(RawSection::Module(module));
    }

    pub(crate) fn core_modules(&self) -> impl Iterator<Item = (u32, &ModuleContext<'a>)> + '_ {
        let mut i = 0;
        self.sections.iter().filter_map(move |s| match s {
            RawSection::Module(m) => Some((inc(&mut i), m)),
            RawSection::Raw(_) => None,
        })
    }

    pub(crate) fn num_core_modules(&self) -> u32 {
        u32::try_from(self.core_modules().count()).unwrap()
    }

    pub(crate) fn inc(&mut self, kind: wasmparser::ComponentExternalKind) {
        match kind {
            wasmparser::ComponentExternalKind::Type => {
                self.inc_types();
            }
            wasmparser::ComponentExternalKind::Instance => {
                self.inc_instances();
            }
            wasmparser::ComponentExternalKind::Func => {
                self.inc_funcs();
            }
            wasmparser::ComponentExternalKind::Component
            | wasmparser::ComponentExternalKind::Module
            | wasmparser::ComponentExternalKind::Value => {}
        }
    }

    pub(crate) fn inc_core(&mut self, kind: wasmparser::ExternalKind) {
        match kind {
            wasmparser::ExternalKind::Func | wasmparser::ExternalKind::FuncExact => {
                self.inc_core_funcs();
            }
            wasmparser::ExternalKind::Memory => {
                self.inc_core_memories();
            }
            wasmparser::ExternalKind::Table
            | wasmparser::ExternalKind::Global
            | wasmparser::ExternalKind::Tag => {}
        }
    }

    pub(crate) fn inc_instances(&mut self) -> u32 {
        inc(&mut self.instances)
    }

    pub(crate) fn inc_funcs(&mut self) -> u32 {
        inc(&mut self.funcs)
    }

    pub(crate) fn inc_core_memories(&mut self) -> u32 {
        inc(&mut self.core_memories)
    }

    pub(crate) fn inc_types(&mut self) -> u32 {
        inc(&mut self.types)
    }

    pub(crate) fn inc_core_instances(&mut self) -> u32 {
        inc(&mut self.core_instances)
    }

    pub(crate) fn inc_core_funcs(&mut self) -> u32 {
        inc(&mut self.core_funcs)
    }
}

fn inc(count: &mut u32) -> u32 {
    let current = *count;
    *count += 1;
    current
}
