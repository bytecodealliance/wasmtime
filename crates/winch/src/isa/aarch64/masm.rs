use crate::masm::MacroAssembler as Masm;

#[derive(Default)]
pub(crate) struct MacroAssembler;

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        todo!()
    }

    fn epilogue(&mut self) {
        todo!()
    }

    fn finalize(self) -> Vec<String> {
        todo!()
    }
}
