//! Unwind information for System V ABI (Aarch64).

use crate::isa::aarch64::inst::regs;
use crate::isa::unwind::input;
use crate::isa::unwind::systemv::{RegisterMappingError, UnwindInfo};
use crate::result::CodegenResult;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register};
use regalloc::{Reg, RegClass};

/// Creates a new aarch64 common information entry (CIE).
pub fn create_cie() -> CommonInformationEntry {
    use gimli::write::CallFrameInstruction;

    let mut entry = CommonInformationEntry::new(
        Encoding {
            address_size: 8,
            format: Format::Dwarf32,
            version: 1,
        },
        4,  // Code alignment factor
        -8, // Data alignment factor
        Register(regs::link_reg().get_hw_encoding().into()),
    );

    // Every frame will start with the call frame address (CFA) at SP
    let sp = Register(regs::stack_reg().get_hw_encoding().into());
    entry.add_instruction(CallFrameInstruction::Cfa(sp, 0));

    entry
}

/// Map Cranelift registers to their corresponding Gimli registers.
pub fn map_reg(reg: Reg) -> Result<Register, RegisterMappingError> {
    match reg.get_class() {
        RegClass::I64 => Ok(Register(reg.get_hw_encoding().into())),
        _ => Err(RegisterMappingError::UnsupportedRegisterBank("class?")),
    }
}

pub(crate) fn create_unwind_info(
    unwind: input::UnwindInfo<Reg>,
) -> CodegenResult<Option<UnwindInfo>> {
    struct RegisterMapper;
    impl crate::isa::unwind::systemv::RegisterMapper<Reg> for RegisterMapper {
        fn map(&self, reg: Reg) -> Result<u16, RegisterMappingError> {
            Ok(map_reg(reg)?.0)
        }
        fn sp(&self) -> u16 {
            regs::stack_reg().get_hw_encoding().into()
        }
    }
    let map = RegisterMapper;
    Ok(Some(UnwindInfo::build(unwind, &map)?))
}

#[cfg(test)]
mod tests {
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        types, AbiParam, ExternalName, Function, InstBuilder, Signature, StackSlotData,
        StackSlotKind,
    };
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use gimli::write::Address;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    fn test_simple_func() {
        let isa = lookup(triple!("aarch64"))
            .expect("expect aarch64 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let fde = match context
            .create_unwind_info(isa.as_ref())
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(1234))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(1234), length: 24, lsda: None, instructions: [(4, CfaOffset(16)), (4, Offset(Register(29), -16)), (4, Offset(Register(30), -8)), (8, CfaRegister(Register(29)))] }");
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
        let isa = lookup(triple!("aarch64"))
            .expect("expect aarch64 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_multi_return_function(CallConv::SystemV));

        context.compile(&*isa).expect("expected compilation");

        let fde = match context
            .create_unwind_info(isa.as_ref())
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(4321))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(4321), length: 40, lsda: None, instructions: [(4, CfaOffset(16)), (4, Offset(Register(29), -16)), (4, Offset(Register(30), -8)), (8, CfaRegister(Register(29)))] }");
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
