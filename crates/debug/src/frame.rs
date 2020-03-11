use crate::transform::map_reg;
use std::collections::HashMap;
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::isa::{CallConv, TargetIsa};
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::{FrameLayoutChange, FrameLayouts};

use gimli::write::{
    Address, CallFrameInstruction, CommonInformationEntry as CIEEntry, Error,
    FrameDescriptionEntry as FDEEntry, FrameTable,
};
use gimli::{Encoding, Format, Register, X86_64};

fn to_cfi(
    isa: &dyn TargetIsa,
    change: &FrameLayoutChange,
    cfa_def_reg: &mut Register,
    cfa_def_offset: &mut i32,
) -> Option<CallFrameInstruction> {
    Some(match change {
        FrameLayoutChange::CallFrameAddressAt { reg, offset } => {
            let mapped = match map_reg(isa, *reg) {
                Ok(r) => r,
                Err(_) => return None,
            };
            let offset = (*offset) as i32;
            if mapped != *cfa_def_reg && offset != *cfa_def_offset {
                *cfa_def_reg = mapped;
                *cfa_def_offset = offset;
                CallFrameInstruction::Cfa(mapped, offset)
            } else if offset != *cfa_def_offset {
                *cfa_def_offset = offset;
                CallFrameInstruction::CfaOffset(offset)
            } else if mapped != *cfa_def_reg {
                *cfa_def_reg = mapped;
                CallFrameInstruction::CfaRegister(mapped)
            } else {
                return None;
            }
        }
        FrameLayoutChange::RegAt { reg, cfa_offset } => {
            assert!(cfa_offset % -8 == 0);
            let cfa_offset = *cfa_offset as i32;
            let mapped = match map_reg(isa, *reg) {
                Ok(r) => r,
                Err(_) => return None,
            };
            CallFrameInstruction::Offset(mapped, cfa_offset)
        }
        FrameLayoutChange::ReturnAddressAt { cfa_offset } => {
            assert!(cfa_offset % -8 == 0);
            let cfa_offset = *cfa_offset as i32;
            CallFrameInstruction::Offset(X86_64::RA, cfa_offset)
        }
        FrameLayoutChange::Preserve => CallFrameInstruction::RememberState,
        FrameLayoutChange::Restore => CallFrameInstruction::RestoreState,
    })
}

pub fn get_debug_frame_bytes(
    funcs: &[(*const u8, usize)],
    isa: &dyn TargetIsa,
    layouts: &FrameLayouts,
) -> Result<Option<FrameTable>, Error> {
    // FIXME Only x86-64 at this moment.
    if isa.name() != "x86" || isa.pointer_bits() != 64 {
        return Ok(None);
    }

    let address_size = isa.pointer_bytes();
    let encoding = Encoding {
        format: Format::Dwarf64,
        version: 4,
        address_size,
    };

    let mut frames = FrameTable::default();

    let mut cached_cies = HashMap::new();

    for (i, f) in funcs.into_iter().enumerate() {
        let layout = &layouts[DefinedFuncIndex::new(i)];

        // FIXME Can only process functions with SystemV-like prologue.
        if layout.call_conv != CallConv::Fast
            && layout.call_conv != CallConv::Cold
            && layout.call_conv != CallConv::SystemV
        {
            continue;
        }

        // Caching CIE with similar initial_commands.
        let (cie_id, mut cfa_def_reg, mut cfa_def_offset) = {
            use std::collections::hash_map::Entry;
            match cached_cies.entry(&layout.initial_commands) {
                Entry::Occupied(o) => *o.get(),
                Entry::Vacant(v) => {
                    // cfa_def_reg and cfa_def_offset initialized with some random values.
                    let mut cfa_def_reg = X86_64::RA;
                    let mut cfa_def_offset = 0i32;

                    // TODO adjust code_alignment_factor and data_alignment_factor based on ISA.
                    let mut cie = CIEEntry::new(
                        encoding,
                        /* code_alignment_factor = */ 1,
                        /* data_alignment_factor = */ -8,
                        /* return_address_register = */ X86_64::RA,
                    );
                    for cmd in layout.initial_commands.iter() {
                        if let Some(instr) = to_cfi(isa, cmd, &mut cfa_def_reg, &mut cfa_def_offset)
                        {
                            cie.add_instruction(instr);
                        }
                    }
                    let cie_id = frames.add_cie(cie);
                    *v.insert((cie_id, cfa_def_reg, cfa_def_offset))
                }
            }
        };

        let f_len = f.1 as u32;
        let mut fde = FDEEntry::new(
            Address::Symbol {
                symbol: i,
                addend: 0,
            },
            f_len,
        );

        for (offset, cmd) in layout.commands.into_iter() {
            if let Some(instr) = to_cfi(isa, cmd, &mut cfa_def_reg, &mut cfa_def_offset) {
                fde.add_instruction(*offset as u32, instr);
            }
        }

        frames.add_fde(cie_id, fde);
    }

    Ok(Some(frames))
}
