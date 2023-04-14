//! The initial instrumentation pass.

use crate::info::{Module, ModuleContext};
use crate::stack_ext::StackExt;
use wasm_encoder::SectionId;

/// Instrument the input Wasm so that it exports its memories and globals,
/// allowing us to inspect their state after the module is instantiated and
/// initialized.
///
/// For example, given this input module:
///
/// ```wat
/// (module $A
///   (module $B
///     (memory $B_mem)
///     (global $B_glob (mut i32))
///   )
///
///   (instance $x (instantiate $B))
///   (instance $y (instantiate $B))
///
///   (memory $A_mem)
///   (global $A_glob (mut i32))
/// )
/// ```
///
/// this pass will produce the following instrumented module:
///
/// ```wat
/// (module $A
///   (module $B
///     (memory $B_mem)
///     (global $B_glob (mut i32))
///
///     ;; Export all state.
///     (export "__wizer_memory_0" (memory $B_mem))
///     (export "__wizer_global_0" (global $B_glob))
///   )
///
///   (instance $x (instantiate $B))
///   (instance $y (instantiate $B))
///
///   (memory $A_mem)
///   (global $A_glob (mut i32))
///
///   ;; Export of all state (including transitively re-exporting nested
///   ;; instantiations' state).
///   (export "__wizer_memory_0" (memory $A_mem))
///   (export "__wizer_global_0" (global $A_glob))
///   (export "__wizer_instance_0" (instance $x))
///   (export "__wizer_instance_1" (instance $y))
/// )
/// ```
///
/// NB: we re-export nested instantiations as a whole instance export because we
/// can do this without disturbing existing instances' indices. If we were to
/// export their memories and globals individually, that would disturb the
/// modules locally defined memoryies' and globals' indices, which would require
/// rewriting the code section, which would break debug info offsets.
pub(crate) fn instrument(cx: &ModuleContext<'_>) -> Vec<u8> {
    log::debug!("Instrumenting the input Wasm");

    struct StackEntry<'a> {
        /// This entry's module.
        module: Module,

        /// The work-in-progress encoding of the new, instrumented module.
        encoder: wasm_encoder::Module,

        /// Sections in this module info that we are iterating over.
        sections: std::slice::Iter<'a, wasm_encoder::RawSection<'a>>,
    }

    let root = cx.root();
    let mut stack = vec![StackEntry {
        module: root,
        encoder: wasm_encoder::Module::new(),
        sections: root.raw_sections(cx).iter(),
    }];

    loop {
        assert!(!stack.is_empty());

        match stack.top_mut().sections.next() {
            // For the exports section, we need to transitively export internal
            // state so that we can read the initialized state after we call the
            // initialization function.
            Some(section) if section.id == SectionId::Export.into() => {
                let entry = stack.top_mut();
                let mut exports = wasm_encoder::ExportSection::new();

                // First, copy over all the original exports.
                for export in entry.module.exports(cx) {
                    exports.export(
                        export.name,
                        match export.kind {
                            wasmparser::ExternalKind::Func => {
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
                            wasmparser::ExternalKind::Tag => {
                                unreachable!("should have been rejected in validation/parsing")
                            }
                        },
                    );
                }

                // Now export all of this module's defined globals, memories,
                // and instantiations under well-known names so we can inspect
                // them after initialization.
                for (i, (j, _)) in entry.module.defined_globals(cx).enumerate() {
                    let name = format!("__wizer_global_{}", i);
                    exports.export(&name, wasm_encoder::Export::Global(j));
                }
                for (i, (j, _)) in entry.module.defined_memories(cx).enumerate() {
                    let name = format!("__wizer_memory_{}", i);
                    exports.export(&name, wasm_encoder::Export::Memory(j));
                }

                entry.encoder.section(&exports);
            }

            // End of the current module: if this is the root, return the
            // instrumented module, otherwise add it as an entry in its parent's
            // module section.
            None => {
                let entry = stack.pop().unwrap();

                if entry.module.is_root() {
                    assert!(stack.is_empty());
                    return entry.encoder.finish();
                }
            }

            // All other sections don't need instrumentation and can be copied
            // over directly.
            Some(section) => {
                stack.top_mut().encoder.section(section);
            }
        }
    }
}
