//! Final rewrite pass.

use crate::{FuncRenames, SnapshotVal, Wizer, info::ModuleContext, snapshot::Snapshot};
use std::cell::Cell;
use std::convert::TryFrom;
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
use wasm_encoder::{ConstExpr, SectionId};

impl Wizer {
    /// Given the initialized snapshot, rewrite the Wasm so that it is already
    /// initialized.
    ///
    /// When `preserve_instrumentation` is true, the `__wizer_*` exports are
    /// preserved so the output can be snapshotted again.
    pub(crate) fn rewrite(
        &self,
        module: &mut ModuleContext<'_>,
        snapshot: &Snapshot,
        renames: &FuncRenames,
        remove_wasi_initialize: bool,
        preserve_instrumentation: bool,
    ) -> Vec<u8> {
        log::debug!("Rewriting input Wasm to pre-initialized state");

        let mut encoder = wasm_encoder::Module::new();
        let has_wasi_initialize = module.has_wasi_initialize();

        // Encode the initialized data segments from the snapshot rather
        // than the original, uninitialized data segments.
        let add_data_segments = |data_section: &mut wasm_encoder::DataSection| {
            for seg in &snapshot.data_segments {
                let offset = if seg.is64 {
                    ConstExpr::i64_const(seg.offset.cast_signed())
                } else {
                    ConstExpr::i32_const(u32::try_from(seg.offset).unwrap().cast_signed())
                };
                data_section.active(seg.memory_index, &offset, seg.data.iter().copied());
            }
        };

        // There are multiple places were we potentially need to check whether
        // we've added the data section already and if we haven't yet, then do
        // so. For example, the original Wasm might not have a data section at
        // all, and so we have to potentially add it at the end of iterating
        // over the original sections. This closure encapsulates all that
        // add-it-if-we-haven't-already logic in one place.
        let added_data_section = Cell::new(false);

        let add_data_section = |encoder: &mut wasm_encoder::Module| {
            if added_data_section.get() {
                return;
            }
            added_data_section.set(true);
            let mut data_section = wasm_encoder::DataSection::new();
            add_data_segments(&mut data_section);
            encoder.section(&data_section);
        };

        for section in module.raw_sections() {
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
                    assert_eq!(module.defined_memories_len(), snapshot.memory_mins.len());
                    for ((_, mem), new_min) in module
                        .defined_memories()
                        .zip(snapshot.memory_mins.iter().copied())
                    {
                        let mut mem = RoundtripReencoder.memory_type(mem).unwrap();
                        mem.minimum = new_min;
                        memories.memory(mem);
                    }
                    encoder.section(&memories);
                }

                // Encode the initialized global values from the snapshot,
                // rather than the original values.
                s if s.id == u8::from(SectionId::Global) => {
                    let original_globals = wasmparser::GlobalSectionReader::new(
                        wasmparser::BinaryReader::new(s.data, 0),
                    )
                    .unwrap();
                    let mut globals = wasm_encoder::GlobalSection::new();
                    let mut snapshot = snapshot.globals.iter();
                    for ((_, glob_ty, export_name), global) in
                        module.defined_globals().zip(original_globals)
                    {
                        let global = global.unwrap();
                        if export_name.is_some() {
                            // This is a mutable global and it was present in
                            // the snapshot, so translate the snapshot value to
                            // a constant expression and insert it.
                            assert!(glob_ty.mutable);
                            let (_, val) = snapshot.next().unwrap();
                            let init = match val {
                                SnapshotVal::I32(x) => ConstExpr::i32_const(*x),
                                SnapshotVal::I64(x) => ConstExpr::i64_const(*x),
                                SnapshotVal::F32(x) => {
                                    ConstExpr::f32_const(wasm_encoder::Ieee32::new(*x))
                                }
                                SnapshotVal::F64(x) => {
                                    ConstExpr::f64_const(wasm_encoder::Ieee64::new(*x))
                                }
                                SnapshotVal::V128(x) => ConstExpr::v128_const(x.cast_signed()),
                            };
                            let glob_ty = RoundtripReencoder.global_type(glob_ty).unwrap();
                            globals.global(glob_ty, &init);
                        } else {
                            // This global isn't mutable so preserve its value
                            // as-is.
                            assert!(!glob_ty.mutable);
                            RoundtripReencoder
                                .parse_global(&mut globals, global)
                                .unwrap();
                        };
                    }
                    encoder.section(&globals);
                }

                // Remove exports for the wizer initialization
                // function and WASI reactor _initialize function,
                // then perform any requested renames.
                s if s.id == u8::from(SectionId::Export) => {
                    let mut exports = wasm_encoder::ExportSection::new();
                    for export in module.exports() {
                        if (export.name == self.get_init_func() && !self.get_keep_init_func())
                            || (remove_wasi_initialize
                                && has_wasi_initialize
                                && export.name == "_initialize")
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

                        let kind = RoundtripReencoder.export_kind(export.kind).unwrap();
                        exports.export(field, kind, export.index);
                    }

                    // Re-add __wizer_* exports so the output remains
                    // instrumentable for future snapshots.
                    if preserve_instrumentation {
                        if let Some(ref global_exports) = module.defined_global_exports {
                            for (idx, name) in global_exports {
                                exports.export(name, wasm_encoder::ExportKind::Global, *idx);
                            }
                        }
                        if let Some(ref memory_exports) = module.defined_memory_exports {
                            for ((mem_idx, _), name) in
                                module.defined_memories().zip(memory_exports)
                            {
                                exports.export(name, wasm_encoder::ExportKind::Memory, mem_idx);
                            }
                        }
                    }

                    encoder.section(&exports);
                }

                // Skip the `start` function -- it's already been run!
                s if s.id == u8::from(SectionId::Start) => {
                    continue;
                }

                // Add the data segments that are being added for the snapshot
                // to the data count section, if present.
                s if s.id == u8::from(SectionId::DataCount) => {
                    let mut data = wasmparser::BinaryReader::new(s.data, 0);
                    let prev = data.read_var_u32().unwrap();
                    assert!(data.eof());
                    encoder.section(&wasm_encoder::DataCountSection {
                        count: prev + u32::try_from(snapshot.data_segments.len()).unwrap(),
                    });
                }

                s if s.id == u8::from(SectionId::Data) => {
                    let mut section = wasm_encoder::DataSection::new();
                    let data = wasmparser::BinaryReader::new(s.data, 0);
                    for data in wasmparser::DataSectionReader::new(data).unwrap() {
                        let data = data.unwrap();
                        match data.kind {
                            // Active data segments, by definition in wasm, are
                            // truncated after instantiation. That means that
                            // for the snapshot all active data segments, which
                            // are already applied, are all turned into empty
                            // passive segments instead.
                            wasmparser::DataKind::Active { .. } => {
                                section.passive([]);
                            }

                            // Passive segments are plumbed through as-is.
                            wasmparser::DataKind::Passive => {
                                section.passive(data.data.iter().copied());
                            }
                        }
                    }

                    // Append all the initializer data segments before adding
                    // the section.
                    add_data_segments(&mut section);
                    encoder.section(&section);
                    added_data_section.set(true);
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
        let mut reader = wasmparser::BinaryReader::new(s.data, 0);
        matches!(reader.read_string(), Ok("name"))
    }
}
