use crate::{
    abi::{align_to, local::LocalSlot, ABISig},
    frame::Frame,
    masm::{MacroAssembler, OperandSize, RegImm},
    regalloc::RegAlloc,
    stack::Stack,
};
use anyhow::Result;
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources};
use wasmtime_environ::WasmType;

/// The code generation context
pub(crate) struct CodeGenContext<'a, M>
where
    M: MacroAssembler,
{
    pub masm: M,
    pub stack: Stack,
    pub frame: &'a Frame,
}

impl<'a, M> CodeGenContext<'a, M>
where
    M: MacroAssembler,
{
    pub fn new(masm: M, stack: Stack, frame: &'a Frame) -> Self {
        Self { masm, stack, frame }
    }
}

/// The code generation abstraction
pub(crate) struct CodeGen<'c, 'a: 'c, M>
where
    M: MacroAssembler,
{
    /// A reference to the function body
    function: &'c mut FunctionBody<'a>,

    /// The word size information, extracted from the current ABI
    word_size: u32,

    /// The ABI-specific representation of the function signature, excluding results
    sig: ABISig,

    /// The code generation context
    pub context: CodeGenContext<'c, M>,

    /// The register allocator
    pub regalloc: RegAlloc,

    /// Function body validator
    pub validator: &'a mut FuncValidator<ValidatorResources>,
}

impl<'c, 'a: 'c, M> CodeGen<'a, 'c, M>
where
    M: MacroAssembler,
{
    pub fn new(
        context: CodeGenContext<'c, M>,
        word_size: u32,
        sig: ABISig,
        function: &'c mut FunctionBody<'a>,
        validator: &'c mut FuncValidator<ValidatorResources>,
        regalloc: RegAlloc,
    ) -> Self {
        Self {
            function,
            word_size,
            sig,
            context,
            regalloc,
            validator,
        }
    }

    /// Emit the function body to machine code
    pub fn emit(&mut self) -> Result<Vec<String>> {
        self.emit_start()
            .and(self.emit_body())
            .and(self.emit_end())?;
        let buf = self.context.masm.finalize();
        let code = Vec::from(buf);
        Ok(code)
    }

    // TODO stack checks
    fn emit_start(&mut self) -> Result<()> {
        self.context.masm.prologue();
        self.context
            .masm
            .reserve_stack(self.context.frame.locals_size);
        Ok(())
    }

    fn emit_body(&mut self) -> Result<()> {
        self.spill_register_arguments();
        self.zero_local_slots();

        let mut ops_reader = self.function.get_operators_reader()?;
        while !ops_reader.eof() {
            ops_reader.visit_with_offset(self)??;
        }
        ops_reader.ensure_end().map_err(|e| e.into())
    }

    // Emit the usual function end instruction sequence
    pub fn emit_end(&mut self) -> Result<()> {
        self.handle_abi_result();
        self.context.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    fn zero_local_slots(&mut self) {
        let range = &self.context.frame.defined_locals_range;
        if range.0.start() == range.0.end() {
            return;
        }

        // Divide the locals range into word-size slots; first ensure that the range limits
        // are word size aligned; since there's no guarantee about their alignment. The aligned "upper"
        // limit should always be less than or equal to the size of the local area, which gets
        // validated when getting the address of a local

        let word_size = self.word_size;
        // If the locals range start is not aligned to the word size, zero the last four bytes
        let range_start = range
            .0
            .start()
            .checked_rem(word_size)
            .map_or(*range.0.start(), |v| {
                if v == 0 {
                    return v;
                }

                let start = range.0.start() + 4;
                let addr = self.context.masm.local_address(&LocalSlot::i32(start));
                self.context
                    .masm
                    .store(RegImm::imm(0), addr, OperandSize::S32);
                start
            });

        // Ensure that the range end is also word-size aligned
        let range_end = align_to(*range.0.end(), word_size);
        // Divide the range into word-size slots
        let slots = (range_end - range_start) / word_size;

        match slots {
            1 => {
                let slot = LocalSlot::i64(range_start + word_size);
                let addr = self.context.masm.local_address(&slot);
                self.context
                    .masm
                    .store(RegImm::imm(0), addr, OperandSize::S64);
            }
            // TODO
            // Add an upper bound to this generation;
            // given a considerably large amount of slots
            // this will be inefficient
            _ => {
                // Request a gpr and zero it
                let zero = self.regalloc.any_gpr(&mut self.context);
                self.context.masm.zero(zero);
                // store zero in each of the slots in the range
                for step in (range_start..range_end)
                    .into_iter()
                    .step_by(word_size as usize)
                {
                    let slot = LocalSlot::i64(step + word_size);
                    let addr = self.context.masm.local_address(&slot);
                    self.context
                        .masm
                        .store(RegImm::reg(zero), addr, OperandSize::S64);
                }
                self.regalloc.free_gpr(zero);
            }
        }
    }

    fn spill_register_arguments(&mut self) {
        // TODO
        // Revisit this once the implicit VMContext argument is introduced;
        // when that happens the mapping between local slots and abi args
        // is not going to be symmetric
        self.sig
            .params
            .iter()
            .enumerate()
            .filter(|(_, a)| a.is_reg())
            .for_each(|(index, arg)| {
                let ty = arg.ty();
                let local = self
                    .context
                    .frame
                    .get_local(index as u32)
                    .expect("valid local slot at location");
                let addr = self.context.masm.local_address(local);
                let src = arg
                    .get_reg()
                    .expect("arg should be associated to a register");

                match &ty {
                    WasmType::I32 => self.context.masm.store(src.into(), addr, OperandSize::S32),
                    WasmType::I64 => self.context.masm.store(src.into(), addr, OperandSize::S64),
                    _ => panic!("Unsupported type {}", ty),
                }
            });
    }

    pub fn handle_abi_result(&mut self) {
        if self.sig.result.is_void() {
            return;
        }
        let named_reg = self.sig.result.result_reg();
        let reg = self
            .regalloc
            .pop_to_named_reg(&mut self.context, named_reg, OperandSize::S64);
        self.regalloc.free_gpr(reg);
    }
}
