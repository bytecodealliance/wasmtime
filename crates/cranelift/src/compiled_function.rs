use std::ops::Range;

use crate::{Relocation, mach_reloc_to_reloc, mach_trap_to_trap};
use cranelift_codegen::{
    Final, MachBufferFinalized, MachBufferFrameLayout, MachSrcLoc, ValueLabelsRanges, ir,
    isa::unwind::CfaUnwindInfo, isa::unwind::UnwindInfo,
};
use wasmtime_environ::{
    FilePos, FrameStateSlotBuilder, InstructionAddressMap, PrimaryMap, TrapInformation,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Metadata to translate from binary offsets back to the original
/// location found in the wasm input.
pub struct FunctionAddressMap {
    /// An array of data for the instructions in this function, indicating where
    /// each instruction maps back to in the original function.
    ///
    /// This array is sorted least-to-greatest by the `code_offset` field.
    /// Additionally the span of each `InstructionAddressMap` is implicitly the
    /// gap between it and the next item in the array.
    pub instructions: Box<[InstructionAddressMap]>,

    /// Function's initial offset in the source file, specified in bytes from
    /// the front of the file.
    pub start_srcloc: FilePos,

    /// Function's end offset in the source file, specified in bytes from
    /// the front of the file.
    pub end_srcloc: FilePos,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: u32,
}

/// The metadata for the compiled function.
#[derive(Default)]
pub struct CompiledFunctionMetadata {
    /// The function address map to translate from binary
    /// back to the original source.
    pub address_map: FunctionAddressMap,
    /// The unwind information.
    pub unwind_info: Option<UnwindInfo>,
    /// CFA-based unwind information for DWARF debugging support.
    pub cfa_unwind_info: Option<CfaUnwindInfo>,
    /// Mapping of value labels and their locations.
    pub value_labels_ranges: ValueLabelsRanges,
    /// Start source location.
    pub start_srcloc: FilePos,
    /// End source location.
    pub end_srcloc: FilePos,
}

/// Compiled function: machine code body, jump table offsets, and unwind information.
pub struct CompiledFunction {
    /// The machine code buffer for this function.
    pub buffer: MachBufferFinalized<Final>,
    /// What names each name ref corresponds to.
    name_map: PrimaryMap<ir::UserExternalNameRef, ir::UserExternalName>,
    /// The alignment for the compiled function.
    pub alignment: u32,
    /// The metadata for the compiled function, including unwind information
    /// the function address map.
    metadata: CompiledFunctionMetadata,
    /// Debug metadata for the top-level function's state slot.
    pub debug_slot_descriptor: Option<FrameStateSlotBuilder>,
    /// Debug breakpoint patches: Wasm PC, offset range in buffer.
    pub breakpoint_patch_points: Vec<(u32, Range<u32>)>,
}

impl CompiledFunction {
    /// Creates a [CompiledFunction] from a [`cranelift_codegen::MachBufferFinalized<Final>`]
    /// This function uses the information in the machine buffer to derive the traps and relocations
    /// fields. The compiled function metadata is loaded with the default values.
    pub fn new(
        buffer: MachBufferFinalized<Final>,
        name_map: PrimaryMap<ir::UserExternalNameRef, ir::UserExternalName>,
        alignment: u32,
    ) -> Self {
        let mut this = Self {
            buffer,
            name_map,
            alignment,
            metadata: Default::default(),
            debug_slot_descriptor: None,
            breakpoint_patch_points: vec![],
        };
        this.finalize_breakpoints();

        this
    }

    /// Finalize breakpoint patches: edit the buffer to have NOPs by
    /// default, and place patch data in the debug breakpoint data
    /// tables.
    fn finalize_breakpoints(&mut self) {
        // Traverse debug tags and patchable callsites together. All
        // patchable callsites should have debug tags. Given both, we
        // can know the Wasm PC and we can emit a breakpoint record.
        let mut tags = self.buffer.debug_tags().peekable();
        let mut patchable_callsites = self.buffer.patchable_call_sites().peekable();

        while let (Some(tag), Some(patchable_callsite)) = (tags.peek(), patchable_callsites.peek())
        {
            if tag.offset > patchable_callsite.ret_addr {
                patchable_callsites.next();
                continue;
            }
            if patchable_callsite.ret_addr > tag.offset {
                tags.next();
                continue;
            }
            assert_eq!(tag.offset, patchable_callsite.ret_addr);

            // Tag format used by our Wasm-to-CLIF format is
            // (stackslot, wasm_pc, stack_shape). Taking the
            // second-to-last tag will get the innermost Wasm PC (if
            // there are multiple nested frames due to inlining).
            assert!(tag.tags.len() >= 3);
            let ir::DebugTag::User(wasm_pc) = tag.tags[tag.tags.len() - 2] else {
                panic!("invalid tag")
            };

            let patchable_start = patchable_callsite.ret_addr - patchable_callsite.len;
            let patchable_end = patchable_callsite.ret_addr;

            self.breakpoint_patch_points
                .push((wasm_pc, patchable_start..patchable_end));

            tags.next();
            patchable_callsites.next();
        }
    }

    /// Returns an iterator to the function's relocation information.
    pub fn relocations(&self) -> impl Iterator<Item = Relocation> + '_ {
        self.buffer
            .relocs()
            .iter()
            .map(|r| mach_reloc_to_reloc(r, &self.name_map))
    }

    /// Returns an iterator to the function's trap information.
    pub fn traps(&self) -> impl Iterator<Item = TrapInformation> + '_ {
        self.buffer.traps().iter().filter_map(mach_trap_to_trap)
    }

    /// Get the function's address map from the metadata.
    pub fn address_map(&self) -> &FunctionAddressMap {
        &self.metadata.address_map
    }

    /// Create and return the compiled function address map from the original source offset
    /// and length.
    pub fn set_address_map(&mut self, offset: u32, length: u32, with_instruction_addresses: bool) {
        assert!((offset + length) <= u32::max_value());
        let len = self.buffer.data().len();
        let srclocs = self
            .buffer
            .get_srclocs_sorted()
            .into_iter()
            .map(|&MachSrcLoc { start, end, loc }| (loc, start, (end - start)));
        let instructions = if with_instruction_addresses {
            collect_address_maps(len.try_into().unwrap(), srclocs)
        } else {
            Default::default()
        };
        let start_srcloc = FilePos::new(offset);
        let end_srcloc = FilePos::new(offset + length);

        let address_map = FunctionAddressMap {
            instructions: instructions.into(),
            start_srcloc,
            end_srcloc,
            body_offset: 0,
            body_len: len.try_into().unwrap(),
        };

        self.metadata.address_map = address_map;
    }

    /// Get a reference to the unwind information from the
    /// function's metadata.
    pub fn unwind_info(&self) -> Option<&UnwindInfo> {
        self.metadata.unwind_info.as_ref()
    }

    /// Get a reference to the compiled function metadata.
    pub fn metadata(&self) -> &CompiledFunctionMetadata {
        &self.metadata
    }

    /// Set the value labels ranges in the function's metadata.
    pub fn set_value_labels_ranges(&mut self, ranges: ValueLabelsRanges) {
        self.metadata.value_labels_ranges = ranges;
    }

    /// Set the unwind info in the function's metadata.
    pub fn set_unwind_info(&mut self, unwind: UnwindInfo) {
        self.metadata.unwind_info = Some(unwind);
    }

    /// Set the CFA-based unwind info in the function's metadata.
    pub fn set_cfa_unwind_info(&mut self, unwind: CfaUnwindInfo) {
        self.metadata.cfa_unwind_info = Some(unwind);
    }

    /// Returns the frame-layout metadata for this function.
    pub fn frame_layout(&self) -> &MachBufferFrameLayout {
        self.buffer
            .frame_layout()
            .expect("Single-function MachBuffer must have frame layout information")
    }

    /// Returns an iterator over breakpoint patches for this function.
    ///
    /// Each tuple is (wasm PC, buffer offset range).
    pub fn breakpoint_patches(&self) -> impl Iterator<Item = (u32, Range<u32>)> + '_ {
        self.breakpoint_patch_points.iter().cloned()
    }
}

// Collects an iterator of `InstructionAddressMap` into a `Vec` for insertion
// into a `FunctionAddressMap`. This will automatically coalesce adjacent
// instructions which map to the same original source position.
fn collect_address_maps(
    code_size: u32,
    iter: impl IntoIterator<Item = (ir::SourceLoc, u32, u32)>,
) -> Vec<InstructionAddressMap> {
    let mut iter = iter.into_iter();
    let (mut cur_loc, mut cur_offset, mut cur_len) = match iter.next() {
        Some(i) => i,
        None => return Vec::new(),
    };
    let mut ret = Vec::new();
    for (loc, offset, len) in iter {
        // If this instruction is adjacent to the previous and has the same
        // source location then we can "coalesce" it with the current
        // instruction.
        if cur_offset + cur_len == offset && loc == cur_loc {
            cur_len += len;
            continue;
        }

        // Push an entry for the previous source item.
        ret.push(InstructionAddressMap {
            srcloc: cvt(cur_loc),
            code_offset: cur_offset,
        });
        // And push a "dummy" entry if necessary to cover the span of ranges,
        // if any, between the previous source offset and this one.
        if cur_offset + cur_len != offset {
            ret.push(InstructionAddressMap {
                srcloc: FilePos::default(),
                code_offset: cur_offset + cur_len,
            });
        }
        // Update our current location to get extended later or pushed on at
        // the end.
        cur_loc = loc;
        cur_offset = offset;
        cur_len = len;
    }
    ret.push(InstructionAddressMap {
        srcloc: cvt(cur_loc),
        code_offset: cur_offset,
    });
    if cur_offset + cur_len != code_size {
        ret.push(InstructionAddressMap {
            srcloc: FilePos::default(),
            code_offset: cur_offset + cur_len,
        });
    }

    return ret;

    fn cvt(loc: ir::SourceLoc) -> FilePos {
        if loc.is_default() {
            FilePos::default()
        } else {
            FilePos::new(loc.bits())
        }
    }
}
