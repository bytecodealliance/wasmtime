//! Support for FDE data generation.

use crate::binemit::{FrameUnwindOffset, FrameUnwindSink, Reloc};
use crate::ir::{FrameLayoutChange, Function};
use crate::isa::{CallConv, RegUnit, TargetIsa};
use alloc::vec::Vec;
use core::convert::TryInto;
use gimli::write::{
    Address, CallFrameInstruction, CommonInformationEntry, EhFrame, EndianVec,
    FrameDescriptionEntry, FrameTable, Result, Writer,
};
use gimli::{Encoding, Format, LittleEndian, Register, X86_64};

pub type FDERelocEntry = (FrameUnwindOffset, Reloc);

const FUNCTION_ENTRY_ADDRESS: Address = Address::Symbol {
    symbol: 0,
    addend: 0,
};

#[derive(Clone)]
struct FDEWriter {
    vec: EndianVec<LittleEndian>,
    relocs: Vec<FDERelocEntry>,
}

impl FDEWriter {
    fn new() -> Self {
        Self {
            vec: EndianVec::new(LittleEndian),
            relocs: Vec::new(),
        }
    }
    fn into_vec_and_relocs(self) -> (Vec<u8>, Vec<FDERelocEntry>) {
        (self.vec.into_vec(), self.relocs)
    }
}

impl Writer for FDEWriter {
    type Endian = LittleEndian;
    fn endian(&self) -> Self::Endian {
        LittleEndian
    }
    fn len(&self) -> usize {
        self.vec.len()
    }
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.vec.write(bytes)
    }
    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> Result<()> {
        self.vec.write_at(offset, bytes)
    }
    fn write_address(&mut self, address: Address, size: u8) -> Result<()> {
        match address {
            Address::Constant(_) => self.vec.write_address(address, size),
            Address::Symbol { .. } => {
                assert_eq!(address, FUNCTION_ENTRY_ADDRESS);
                let rt = match size {
                    4 => Reloc::Abs4,
                    8 => Reloc::Abs8,
                    _ => {
                        panic!("Unexpected address size at FDEWriter::write_address");
                    }
                };
                self.relocs.push((self.vec.len().try_into().unwrap(), rt));
                self.vec.write_udata(0, size)
            }
        }
    }
}

fn return_address_reg(isa: &dyn TargetIsa) -> Register {
    assert!(isa.name() == "x86" && isa.pointer_bits() == 64);
    X86_64::RA
}

fn map_reg(isa: &dyn TargetIsa, reg: RegUnit) -> Register {
    assert!(isa.name() == "x86" && isa.pointer_bits() == 64);
    // Mapping from https://github.com/bytecodealliance/cranelift/pull/902 by @iximeow
    const X86_GP_REG_MAP: [gimli::Register; 16] = [
        X86_64::RAX,
        X86_64::RCX,
        X86_64::RDX,
        X86_64::RBX,
        X86_64::RSP,
        X86_64::RBP,
        X86_64::RSI,
        X86_64::RDI,
        X86_64::R8,
        X86_64::R9,
        X86_64::R10,
        X86_64::R11,
        X86_64::R12,
        X86_64::R13,
        X86_64::R14,
        X86_64::R15,
    ];
    const X86_XMM_REG_MAP: [gimli::Register; 16] = [
        X86_64::XMM0,
        X86_64::XMM1,
        X86_64::XMM2,
        X86_64::XMM3,
        X86_64::XMM4,
        X86_64::XMM5,
        X86_64::XMM6,
        X86_64::XMM7,
        X86_64::XMM8,
        X86_64::XMM9,
        X86_64::XMM10,
        X86_64::XMM11,
        X86_64::XMM12,
        X86_64::XMM13,
        X86_64::XMM14,
        X86_64::XMM15,
    ];
    let reg_info = isa.register_info();
    let bank = reg_info.bank_containing_regunit(reg).unwrap();
    match bank.name {
        "IntRegs" => {
            // x86 GP registers have a weird mapping to DWARF registers, so we use a
            // lookup table.
            X86_GP_REG_MAP[(reg - bank.first_unit) as usize]
        }
        "FloatRegs" => X86_XMM_REG_MAP[(reg - bank.first_unit) as usize],
        _ => {
            panic!("unsupported register bank: {}", bank.name);
        }
    }
}

fn to_cfi(
    isa: &dyn TargetIsa,
    change: &FrameLayoutChange,
    cfa_def_reg: &mut Register,
    cfa_def_offset: &mut i32,
) -> Option<CallFrameInstruction> {
    Some(match change {
        FrameLayoutChange::CallFrameAddressAt { reg, offset } => {
            let mapped = map_reg(isa, *reg);
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
            let mapped = map_reg(isa, *reg);
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

/// Creates FDE structure from FrameLayout.
pub fn emit_fde(func: &Function, isa: &dyn TargetIsa, sink: &mut dyn FrameUnwindSink) {
    assert!(isa.name() == "x86");

    // Expecting function with System V prologue
    assert!(
        func.signature.call_conv == CallConv::Fast
            || func.signature.call_conv == CallConv::Cold
            || func.signature.call_conv == CallConv::SystemV
    );

    assert!(func.frame_layout.is_some(), "expected func.frame_layout");
    let frame_layout = func.frame_layout.as_ref().unwrap();

    let mut ebbs = func.layout.ebbs().collect::<Vec<_>>();
    ebbs.sort_by_key(|ebb| func.offsets[*ebb]); // Ensure inst offsets always increase

    let encinfo = isa.encoding_info();
    let mut last_offset = 0;
    let mut changes = Vec::new();
    for ebb in ebbs {
        for (offset, inst, size) in func.inst_offsets(ebb, &encinfo) {
            let address_offset = (offset + size) as usize;
            assert!(last_offset <= address_offset);
            if let Some(cmds) = frame_layout.instructions.get(&inst) {
                for cmd in cmds.iter() {
                    changes.push((address_offset, *cmd));
                }
            }
            last_offset = address_offset;
        }
    }

    let len = last_offset as u32;

    let word_size = isa.pointer_bytes() as i32;

    let encoding = Encoding {
        format: Format::Dwarf32,
        version: 1,
        address_size: word_size as u8,
    };
    let mut frames = FrameTable::default();

    let mut cfa_def_reg = return_address_reg(isa);
    let mut cfa_def_offset = 0i32;

    let mut cie = CommonInformationEntry::new(
        encoding,
        /* code_alignment_factor = */ 1,
        /* data_alignment_factor = */ -word_size as i8,
        return_address_reg(isa),
    );
    for ch in frame_layout.initial.iter() {
        if let Some(cfi) = to_cfi(isa, ch, &mut cfa_def_reg, &mut cfa_def_offset) {
            cie.add_instruction(cfi);
        }
    }

    let cie_id = frames.add_cie(cie);

    let mut fde = FrameDescriptionEntry::new(FUNCTION_ENTRY_ADDRESS, len);

    for (addr, ch) in changes.iter() {
        if let Some(cfi) = to_cfi(isa, ch, &mut cfa_def_reg, &mut cfa_def_offset) {
            fde.add_instruction((*addr) as u32, cfi);
        }
    }

    frames.add_fde(cie_id, fde);

    let mut eh_frame = EhFrame::from(FDEWriter::new());
    frames.write_eh_frame(&mut eh_frame).unwrap();

    let (bytes, relocs) = eh_frame.clone().into_vec_and_relocs();

    let unwind_start = sink.len();
    sink.bytes(&bytes);

    for (off, r) in relocs {
        sink.reloc(r, off + unwind_start);
    }

    let cie_len = u32::from_le_bytes(bytes.as_slice()[..4].try_into().unwrap());
    let fde_offset = cie_len as usize + 4;
    sink.set_entry_offset(unwind_start + fde_offset);

    // Need 0 marker for GCC unwind to end FDE "list".
    sink.bytes(&[0, 0, 0, 0]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binemit::{FrameUnwindOffset, Reloc};
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        types, AbiParam, ExternalName, InstBuilder, Signature, StackSlotData, StackSlotKind,
        TrapCode,
    };
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use std::str::FromStr;
    use target_lexicon::triple;

    struct SimpleUnwindSink(pub Vec<u8>, pub usize, pub Vec<(Reloc, usize)>);
    impl FrameUnwindSink for SimpleUnwindSink {
        fn len(&self) -> FrameUnwindOffset {
            self.0.len()
        }
        fn bytes(&mut self, b: &[u8]) {
            self.0.extend_from_slice(b);
        }
        fn reloc(&mut self, r: Reloc, off: FrameUnwindOffset) {
            self.2.push((r, off));
        }
        fn set_entry_offset(&mut self, off: FrameUnwindOffset) {
            self.1 = off;
        }
    }

    #[test]
    fn test_simple_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));
        context.func.collect_frame_layout_info();

        context.compile(&*isa).expect("expected compilation");

        let mut sink = SimpleUnwindSink(Vec::new(), 0, Vec::new());
        emit_fde(&context.func, &*isa, &mut sink);

        assert_eq!(
            sink.0,
            vec![
                20, 0, 0, 0, // CIE len
                0, 0, 0, 0,   // CIE marker
                1,   // version
                0,   // augmentation string
                1,   // code aligment = 1
                120, // data alignment = -8
                16,  // RA = r16
                0x0c, 0x07, 0x08, // DW_CFA_def_cfa r7, 8
                0x90, 0x01, //  DW_CFA_offset r16, -8 * 1
                0, 0, 0, 0, 0, 0, // padding
                36, 0, 0, 0, // FDE len
                28, 0, 0, 0, // CIE offset
                0, 0, 0, 0, 0, 0, 0, 0, // addr reloc
                16, 0, 0, 0, 0, 0, 0, 0,    // function length
                0x42, // DW_CFA_advance_loc 2
                0x0e, 0x10, // DW_CFA_def_cfa_offset 16
                0x86, 0x02, // DW_CFA_offset r6, -8 * 2
                0x43, // DW_CFA_advance_loc 3
                0x0d, 0x06, // DW_CFA_def_cfa_register
                0x4a, // DW_CFA_advance_loc 10
                0x0c, 0x07, 0x08, // DW_CFA_def_cfa r7, 8
                0, 0, 0, 0, // padding
                0, 0, 0, 0, // End of FDEs
            ]
        );
        assert_eq!(sink.1, 24);
        assert_eq!(sink.2.len(), 1);
    }

    fn create_function(call_conv: CallConv, stack_slot: Option<StackSlotData>) -> Function {
        let mut func =
            Function::with_name_signature(ExternalName::user(0, 0), Signature::new(call_conv));

        let ebb0 = func.dfg.make_ebb();
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_ebb(ebb0);
        pos.ins().return_(&[]);

        if let Some(stack_slot) = stack_slot {
            func.stack_slots.push(stack_slot);
        }

        func
    }

    #[test]
    fn test_multi_return_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_multi_return_function(CallConv::SystemV));
        context.func.collect_frame_layout_info();

        context.compile(&*isa).expect("expected compilation");

        let mut sink = SimpleUnwindSink(Vec::new(), 0, Vec::new());
        emit_fde(&context.func, &*isa, &mut sink);

        assert_eq!(
            sink.0,
            vec![
                20, 0, 0, 0, // CIE len
                0, 0, 0, 0,   // CIE marker
                1,   // version
                0,   // augmentation string
                1,   // code aligment = 1
                120, // data alignment = -8
                16,  // RA = r16
                0x0c, 0x07, 0x08, // DW_CFA_def_cfa r7, 8
                0x90, 0x01, //  DW_CFA_offset r16, -8 * 1
                0, 0, 0, 0, 0, 0, // padding
                36, 0, 0, 0, // FDE len
                28, 0, 0, 0, // CIE offset
                0, 0, 0, 0, 0, 0, 0, 0, // addr reloc
                15, 0, 0, 0, 0, 0, 0, 0,    // function length
                0x42, // DW_CFA_advance_loc 2
                0x0e, 0x10, // DW_CFA_def_cfa_offset 16
                0x86, 0x02, // DW_CFA_offset r6, -8 * 2
                0x43, // DW_CFA_advance_loc 3
                0x0d, 0x06, // DW_CFA_def_cfa_register
                0x47, // DW_CFA_advance_loc 10
                0x0a, // DW_CFA_preserve_state
                0x0c, 0x07, 0x08, // DW_CFA_def_cfa r7, 8
                0x41, // DW_CFA_advance_loc 1
                0x0b, // DW_CFA_restore_state
                // NOTE: no additional CFA directives -- DW_CFA_restore_state
                // is done before trap and it is last instruction in the function.
                0, // padding
                0, 0, 0, 0, // End of FDEs
            ]
        );
        assert_eq!(sink.1, 24);
        assert_eq!(sink.2.len(), 1);
    }

    fn create_multi_return_function(call_conv: CallConv) -> Function {
        let mut sig = Signature::new(call_conv);
        sig.params.push(AbiParam::new(types::I32));
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);

        let ebb0 = func.dfg.make_ebb();
        let v0 = func.dfg.append_ebb_param(ebb0, types::I32);
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_ebb(ebb0);
        pos.ins().brnz(v0, ebb2, &[]);
        pos.ins().jump(ebb1, &[]);

        pos.insert_ebb(ebb1);
        pos.ins().return_(&[]);

        pos.insert_ebb(ebb2);
        pos.ins().trap(TrapCode::User(0));

        func
    }
}
