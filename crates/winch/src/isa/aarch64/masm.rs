use crate::{
    abi::{addressing_mode::Address, local::LocalSlot, ABI},
    frame::DefinedLocalsRange,
    masm::{MacroAssembler as Masm, OperandSize, RegImm},
};

#[derive(Default)]
pub(crate) struct MacroAssembler;

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        todo!()
    }

    fn epilogue(&mut self) {
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

    fn zero_local_slots<A: ABI>(&mut self, range: &DefinedLocalsRange, a: &A) {
        todo!()
    }

    fn finalize(&mut self) -> &[String] {
        todo!()
    }
}
