use crate::abi::{ABISig, ABI};
use crate::frame::Frame;
use crate::masm::{MacroAssembler, OperandSize};
use anyhow::Result;
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources};
use wasmtime_environ::{FunctionBodyData, WasmFuncType, WasmType};

/// Per-function compilation environment
pub(crate) struct CompilationEnv<'x, 'a: 'x, A: ABI, C: MacroAssembler> {
    /// A reference to the function body
    function: &'x mut FunctionBody<'a>,

    /// The stack frame handler for the current function
    frame: Frame,

    /// The ABI used in this compilation environment
    abi: A,

    /// The macroassembler used in this compilation environment
    masm: C,

    /// The ABI-specific representation of the function signature
    sig: ABISig,

    /// Wasm validator
    validator: &'x mut FuncValidator<ValidatorResources>,
}

impl<'x, 'a: 'x, A: ABI, C: MacroAssembler> CompilationEnv<'a, 'x, A, C> {
    /// Allocate a new compilation environment
    pub fn new(
        signature: &WasmFuncType,
        function: &'x mut FunctionBody<'a>,
        validator: &'x mut FuncValidator<ValidatorResources>,
        abi: A,
        masm: C,
    ) -> Result<Self> {
        let sig = abi.sig(&signature);
        let frame = Frame::new(&sig, function, validator, &abi)?;

        Ok(Self {
            abi,
            sig,
            frame,
            function,
            masm,
            validator,
        })
    }

    // TODO Order
    // 1. Emit prologue
    //   1.1 Without any stack checks, the idea is to get to code emission and have an initial pass on the Assembler
    //   1.2 Register input spilling
    // 2. Function body
    // 3. Epilogue
    // 4. Stack checks
    /// Emit the function body to machine code
    pub fn emit(&mut self) -> Result<Vec<String>> {
        self.emit_start()
            .and(self.emit_body())
            .and(self.emit_end())?;
        let buf = self.masm.finalize();
        let code = Vec::from(buf);
        Ok(code)
    }

    // Emit the usual function start instruction sequence
    // for the current function:
    // 1. Prologue
    // 2. Stack checks
    // 3. Stack allocation
    fn emit_start(&mut self) -> Result<()> {
        self.masm.prologue();
        self.masm.reserve_stack(self.frame.locals_size);
        Ok(())
    }

    // 1. Perform input register spilling
    // 2. Emit machine code per instruction
    fn emit_body(&mut self) -> Result<()> {
        self.spill_register_arguments();
        self.masm
            .zero_local_slots(&self.frame.defined_locals_range, &self.abi);
        Ok(())
    }

    // Emit the usual function end instruction sequence
    fn emit_end(&mut self) -> Result<()> {
        Ok(())
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
                // TODO
                // Move the calculation of the local from slot
                // to the frame
                let local = self
                    .frame
                    .locals
                    .get(index)
                    .expect("valid local slot at location");
                let addr = self.masm.local_address(local);
                let src = arg
                    .get_reg()
                    .expect("arg should be associated to a register");

                match &ty {
                    WasmType::I32 => self.masm.store(src.into(), addr, OperandSize::S32),
                    WasmType::I64 => self.masm.store(src.into(), addr, OperandSize::S64),
                    _ => panic!("Unsupported type {}", ty),
                }
            });
    }
}
