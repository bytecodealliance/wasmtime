use crate::{
    abi::{addressing_mode::Address, local::LocalSlot},
    isa::reg::Reg,
    masm::{MacroAssembler as Masm, OperandSize, RegImm},
};

#[derive(Default)]
pub(crate) struct MacroAssembler;

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        todo!()
    }

    fn epilogue(&mut self, locals_size: u32) {
        todo!()
    }

    fn reserve_stack(&mut self, bytes: u32) {
        todo!()
    }

    fn local_address(&mut self, local: &LocalSlot) -> Address {
        todo!()
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
        todo!()
    }

    fn sp_offset(&mut self) -> u32 {
        0u32
    }

    fn finalize(&mut self) -> &[String] {
        todo!()
    }

    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize) {
        todo!()
    }

    fn add(&mut self, src: RegImm, dst: RegImm, size: OperandSize) {
        todo!()
    }

    fn zero(&mut self, reg: Reg) {
        todo!()
    }
}
