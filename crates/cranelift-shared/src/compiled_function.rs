use crate::{mach_reloc_to_reloc, mach_trap_to_trap, Relocation};
use cranelift_codegen::{
    ir, ir::UserExternalNameRef, isa::unwind::UnwindInfo, Final, MachBufferFinalized,
    ValueLabelsRanges,
};
use wasmtime_environ::{FilePos, InstructionAddressMap, TrapInformation};

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

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Default)]
pub struct CompiledFunction {
    /// The machine code for this function.
    pub body: Vec<u8>,

    /// The unwind information.
    pub unwind_info: Option<UnwindInfo>,

    /// Information used to translate from binary offsets back to the original
    /// location found in the wasm input.
    pub address_map: FunctionAddressMap,

    /// Metadata about traps in this module, mapping code offsets to the trap
    /// that they may cause.
    pub traps: Vec<TrapInformation>,

    pub relocations: Vec<Relocation>,
    pub value_labels_ranges: ValueLabelsRanges,
    pub sized_stack_slots: ir::StackSlots,
    pub alignment: u32,
}

impl CompiledFunction {
    /// Creates a [CompiledFunction] from a [cranelift_codegen::MachBufferFinalized<Final>]
    /// This function uses the information in the machine buffer to derive the traps and relocations
    /// fields. The rest of the fields are left with their default value.
    pub fn new<F>(buffer: &MachBufferFinalized<Final>, body: Vec<u8>, lookup: &mut F) -> Self
    where
        F: FnMut(UserExternalNameRef) -> (u32, u32),
    {
        let relocations = buffer
            .relocs()
            .into_iter()
            .map(|reloc| mach_reloc_to_reloc(reloc, lookup))
            .collect();
        let traps = buffer.traps().into_iter().map(mach_trap_to_trap).collect();

        Self {
            body,
            relocations,
            traps,
            ..Default::default()
        }
    }

    /// Set the traps of the compiled function from
    /// a [cranelift_codegen::MachBufferFinalized].
    pub fn set_traps(&mut self, buffer: &MachBufferFinalized<Final>) {
        self.traps = buffer.traps().into_iter().map(mach_trap_to_trap).collect();
    }
}
