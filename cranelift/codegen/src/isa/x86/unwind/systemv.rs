//! Unwind information for System V ABI (x86-64).

use crate::ir::Function;
use crate::isa::{
    unwind::systemv::{RegisterMappingError, UnwindInfo},
    RegUnit, TargetIsa,
};
use crate::result::CodegenResult;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register, X86_64};

/// Creates a new x86-64 common information entry (CIE).
pub fn create_cie() -> CommonInformationEntry {
    use gimli::write::CallFrameInstruction;

    let mut entry = CommonInformationEntry::new(
        Encoding {
            address_size: 8,
            format: Format::Dwarf32,
            version: 1,
        },
        1,  // Code alignment factor
        -8, // Data alignment factor
        X86_64::RA,
    );

    // Every frame will start with the call frame address (CFA) at RSP+8
    // It is +8 to account for the push of the return address by the call instruction
    entry.add_instruction(CallFrameInstruction::Cfa(X86_64::RSP, 8));

    // Every frame will start with the return address at RSP (CFA-8 = RSP+8-8 = RSP)
    entry.add_instruction(CallFrameInstruction::Offset(X86_64::RA, -8));

    entry
}

/// Map Cranelift registers to their corresponding Gimli registers.
pub fn map_reg(isa: &dyn TargetIsa, reg: RegUnit) -> Result<Register, RegisterMappingError> {
    if isa.name() != "x86" || isa.pointer_bits() != 64 {
        return Err(RegisterMappingError::UnsupportedArchitecture);
    }

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
    let bank = reg_info
        .bank_containing_regunit(reg)
        .ok_or_else(|| RegisterMappingError::MissingBank)?;
    match bank.name {
        "IntRegs" => {
            // x86 GP registers have a weird mapping to DWARF registers, so we use a
            // lookup table.
            Ok(X86_GP_REG_MAP[(reg - bank.first_unit) as usize])
        }
        "FloatRegs" => Ok(X86_XMM_REG_MAP[(reg - bank.first_unit) as usize]),
        _ => Err(RegisterMappingError::UnsupportedRegisterBank(bank.name)),
    }
}

pub(crate) fn create_unwind_info(
    func: &Function,
    isa: &dyn TargetIsa,
) -> CodegenResult<Option<UnwindInfo>> {
    // Only System V-like calling conventions are supported
    match isa.unwind_info_kind() {
        crate::machinst::UnwindInfoKind::SystemV => {}
        _ => return Ok(None),
    }

    if func.prologue_end.is_none() || isa.name() != "x86" || isa.pointer_bits() != 64 {
        return Ok(None);
    }

    let unwind = match super::create_unwind_info(func, isa)? {
        Some(u) => u,
        None => {
            return Ok(None);
        }
    };

    struct RegisterMapper<'a, 'b>(&'a (dyn TargetIsa + 'b));
    impl<'a, 'b> crate::isa::unwind::systemv::RegisterMapper<RegUnit> for RegisterMapper<'a, 'b> {
        fn map(&self, reg: RegUnit) -> Result<u16, RegisterMappingError> {
            Ok(map_reg(self.0, reg)?.0)
        }
        fn sp(&self) -> u16 {
            X86_64::RSP.0
        }
        fn fp(&self) -> u16 {
            X86_64::RBP.0
        }
    }
    let map = RegisterMapper(isa);

    Ok(Some(UnwindInfo::build(unwind, &map)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        types, AbiParam, ExternalName, InstBuilder, Signature, StackSlotData, StackSlotKind,
    };
    use crate::isa::{lookup_variant, BackendVariant, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use gimli::write::Address;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    fn test_simple_func() {
        let isa = lookup_variant(triple!("x86_64"), BackendVariant::Legacy)
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let fde = match isa
            .create_unwind_info(&context.func)
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(1234))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(1234), length: 16, lsda: None, instructions: [(2, CfaOffset(16)), (2, Offset(Register(6), -16)), (5, CfaRegister(Register(6))), (15, SameValue(Register(6))), (15, Cfa(Register(7), 8))] }");
    }

    fn create_function(call_conv: CallConv, stack_slot: Option<StackSlotData>) -> Function {
        let mut func =
            Function::with_name_signature(ExternalName::user(0, 0), Signature::new(call_conv));

        let block0 = func.dfg.make_block();
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(block0);
        pos.ins().return_(&[]);

        if let Some(stack_slot) = stack_slot {
            func.stack_slots.push(stack_slot);
        }

        func
    }

    #[test]
    fn test_multi_return_func() {
        let isa = lookup_variant(triple!("x86_64"), BackendVariant::Legacy)
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_multi_return_function(CallConv::SystemV));

        context.compile(&*isa).expect("expected compilation");

        let fde = match isa
            .create_unwind_info(&context.func)
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(4321))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(4321), length: 16, lsda: None, instructions: [(2, CfaOffset(16)), (2, Offset(Register(6), -16)), (5, CfaRegister(Register(6))), (12, RememberState), (12, SameValue(Register(6))), (12, Cfa(Register(7), 8)), (13, RestoreState), (15, SameValue(Register(6))), (15, Cfa(Register(7), 8))] }");
    }

    fn create_multi_return_function(call_conv: CallConv) -> Function {
        let mut sig = Signature::new(call_conv);
        sig.params.push(AbiParam::new(types::I32));
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);

        let block0 = func.dfg.make_block();
        let v0 = func.dfg.append_block_param(block0, types::I32);
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(block0);
        pos.ins().brnz(v0, block2, &[]);
        pos.ins().jump(block1, &[]);

        pos.insert_block(block1);
        pos.ins().return_(&[]);

        pos.insert_block(block2);
        pos.ins().return_(&[]);

        func
    }
}
