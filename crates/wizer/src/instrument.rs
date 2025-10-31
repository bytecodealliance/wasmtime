//! The initial instrumentation pass.

use crate::info::ModuleContext;
use wasm_encoder::SectionId;
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};

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
pub(crate) fn instrument(module: &mut ModuleContext<'_>) -> Vec<u8> {
    log::debug!("Instrumenting the input Wasm");

    let mut encoder = wasm_encoder::Module::new();
    let mut defined_global_exports = Vec::new();
    let mut defined_memory_exports = Vec::new();

    for section in module.raw_sections() {
        match section.id {
            // For the exports section, we need to transitively export internal
            // state so that we can read the initialized state after we call the
            // initialization function.
            id if id == u8::from(SectionId::Export) => {
                let mut exports = wasm_encoder::ExportSection::new();

                // First, copy over all the original exports.
                for export in module.exports() {
                    RoundtripReencoder
                        .parse_export(&mut exports, *export)
                        .unwrap();
                }

                // Now export all of this module's defined globals, memories,
                // and instantiations under well-known names so we can inspect
                // them after initialization.
                for (i, ty, _) in module.defined_globals() {
                    if !ty.mutable {
                        continue;
                    }
                    let name = format!("__wizer_global_{i}");
                    exports.export(&name, wasm_encoder::ExportKind::Global, i);
                    defined_global_exports.push((i, name));
                }
                for (i, (j, _)) in module.defined_memories().enumerate() {
                    let name = format!("__wizer_memory_{i}");
                    exports.export(&name, wasm_encoder::ExportKind::Memory, j);
                    defined_memory_exports.push(name);
                }

                encoder.section(&exports);
            }

            // All other sections don't need instrumentation and can be copied
            // over directly.
            _other => {
                encoder.section(section);
            }
        }
    }

    module.defined_global_exports = Some(defined_global_exports);
    module.defined_memory_exports = Some(defined_memory_exports);

    encoder.finish()
}
