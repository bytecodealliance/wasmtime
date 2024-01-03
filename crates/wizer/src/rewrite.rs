//! Final rewrite pass.

use crate::{
    info::ModuleContext, snapshot::Snapshot, translate, FuncRenames, Wizer, DEFAULT_KEEP_INIT_FUNC,
};
use std::convert::TryFrom;
use wasm_encoder::{ConstExpr, SectionId};

impl Wizer {
    /// Given the initialized snapshot, rewrite the Wasm so that it is already
    /// initialized.
    ///
    pub(crate) fn rewrite(
        &self,
        cx: &mut ModuleContext<'_>,
        store: &crate::Store,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        has_wasi_initialize: bool,
    ) -> Vec<u8> {
        log::debug!("Rewriting input Wasm to pre-initialized state");

        let mut encoder = wasm_encoder::Module::new();
        let module = cx.root();

        // Encode the initialized data segments from the snapshot rather
        // than the original, uninitialized data segments.
        let mut data_section = if snapshot.data_segments.is_empty() {
            None
        } else {
            let mut data_section = wasm_encoder::DataSection::new();
            for seg in &snapshot.data_segments {
                data_section.active(
                    seg.memory_index,
                    &ConstExpr::i32_const(seg.offset as i32),
                    seg.data(store).iter().copied(),
                );
            }
            Some(data_section)
        };

        // There are multiple places were we potentially need to check whether
        // we've added the data section already and if we haven't yet, then do
        // so. For example, the original Wasm might not have a data section at
        // all, and so we have to potentially add it at the end of iterating
        // over the original sections. This closure encapsulates all that
        // add-it-if-we-haven't-already logic in one place.
        let mut add_data_section = |module: &mut wasm_encoder::Module| {
            if let Some(data_section) = data_section.take() {
                module.section(&data_section);
            }
        };

        for section in module.raw_sections(cx) {
            match section {
                // Some tools expect the name custom section to come last, even
                // though custom sections are allowed in any order. Therefore,
                // make sure we've added our data section by now.
                s if is_name_section(s) => {
                    add_data_section(&mut encoder);
                    encoder.section(s);
                }

                // For the memory section, we update the minimum size of each
                // defined memory to the snapshot's initialized size for that
                // memory.
                s if s.id == u8::from(SectionId::Memory) => {
                    let mut memories = wasm_encoder::MemorySection::new();
                    assert_eq!(module.defined_memories_len(cx), snapshot.memory_mins.len());
                    for ((_, mem), new_min) in module
                        .defined_memories(cx)
                        .zip(snapshot.memory_mins.iter().copied())
                    {
                        let mut mem = translate::memory_type(mem);
                        mem.minimum = new_min;
                        memories.memory(mem);
                    }
                    encoder.section(&memories);
                }

                // Encode the initialized global values from the snapshot,
                // rather than the original values.
                s if s.id == u8::from(SectionId::Global) => {
                    let mut globals = wasm_encoder::GlobalSection::new();
                    for ((_, glob_ty), val) in
                        module.defined_globals(cx).zip(snapshot.globals.iter())
                    {
                        let glob_ty = translate::global_type(glob_ty);
                        globals.global(
                            glob_ty,
                            &match val {
                                wasmtime::Val::I32(x) => ConstExpr::i32_const(*x),
                                wasmtime::Val::I64(x) => ConstExpr::i64_const(*x),
                                wasmtime::Val::F32(x) => ConstExpr::f32_const(f32::from_bits(*x)),
                                wasmtime::Val::F64(x) => ConstExpr::f64_const(f64::from_bits(*x)),
                                wasmtime::Val::V128(x) => {
                                    ConstExpr::v128_const(x.as_u128() as i128)
                                }
                                _ => unreachable!(),
                            },
                        );
                    }
                    encoder.section(&globals);
                }

                // Remove exports for the wizer initialization
                // function and WASI reactor _initialize function,
                // then perform any requested renames.
                s if s.id == u8::from(SectionId::Export) => {
                    let mut exports = wasm_encoder::ExportSection::new();
                    for export in module.exports(cx) {
                        if (export.name == self.init_func
                            && !self.keep_init_func.unwrap_or(DEFAULT_KEEP_INIT_FUNC))
                            || (has_wasi_initialize && export.name == "_initialize")
                        {
                            continue;
                        }

                        if !renames.rename_src_to_dst.contains_key(export.name)
                            && renames.rename_dsts.contains(export.name)
                        {
                            // A rename overwrites this export, and it is not
                            // renamed to another export, so skip it.
                            continue;
                        }

                        let field = renames
                            .rename_src_to_dst
                            .get(export.name)
                            .map_or(export.name, |f| f.as_str());

                        let kind = translate::export(export.kind);
                        exports.export(field, kind, export.index);
                    }
                    encoder.section(&exports);
                }

                // Skip the `start` function -- it's already been run!
                s if s.id == u8::from(SectionId::Start) => {
                    continue;
                }

                s if s.id == u8::from(SectionId::DataCount) => {
                    encoder.section(&wasm_encoder::DataCountSection {
                        count: u32::try_from(snapshot.data_segments.len()).unwrap(),
                    });
                }

                s if s.id == u8::from(SectionId::Data) => {
                    // TODO: supporting bulk memory will require copying over
                    // any passive and declared segments.
                    add_data_section(&mut encoder);
                }

                s => {
                    encoder.section(s);
                }
            }
        }

        // Make sure that we've added our data section to the module.
        add_data_section(&mut encoder);
        encoder.finish()
    }
}

fn is_name_section(s: &wasm_encoder::RawSection) -> bool {
    s.id == u8::from(SectionId::Custom) && {
        let mut reader = wasmparser::BinaryReader::new(s.data);
        matches!(reader.read_string(), Ok("name"))
    }
}
