use crate::abi::{align_to, local::LocalSlot, ABISig, ABI};
use crate::frame::Frame;
use crate::isa::reg::Reg;
use crate::masm::{MacroAssembler, OperandSize, RegImm};
use crate::regset::RegSet;
use crate::stack::Stack;
use anyhow::Result;
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources};
use wasmtime_environ::{WasmFuncType, WasmType};

/// Per-function compilation environment
pub(crate) struct CompilationEnv<'x, 'a: 'x, A: ABI, C: MacroAssembler> {
    /// A reference to the function body
    function: &'x mut FunctionBody<'a>,

    /// The stack frame handler for the current function
    frame: Frame,

    /// The ABI used in this compilation environment
    abi: A,

    /// The macroassembler used in this compilation environment
    pub masm: C,

    /// The ABI-specific representation of the function signature, excluding results
    sig: ABISig,

    /// The register for register allocation
    regset: RegSet,

    /// The shadow stack
    pub stack: Stack,

    /// Function body validator
    pub validator: &'a mut FuncValidator<ValidatorResources>,
}

impl<'x, 'a: 'x, A: ABI, C: MacroAssembler> CompilationEnv<'a, 'x, A, C> {
    /// Allocate a new compilation environment
    pub fn new(
        signature: &WasmFuncType,
        function: &'x mut FunctionBody<'a>,
        validator: &'x mut FuncValidator<ValidatorResources>,
        abi: A,
        masm: C,
        regset: RegSet,
    ) -> Result<Self> {
        let sig = abi.sig(&signature);
        let stack = Default::default();
        let frame = Frame::new(&sig, function, validator, &abi)?;

        Ok(Self {
            abi,
            sig,
            frame,
            function,
            masm,
            validator,
            stack,
            regset,
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
        self.zero_local_slots();

        let mut ops_reader = self.function.get_operators_reader()?;
        while !ops_reader.eof() {
            ops_reader.visit_with_offset(self);
        }
        ops_reader.ensure_end()?;

        self.ensure_return();
        Ok(())
    }

    // Emit the usual function end instruction sequence
    pub fn emit_end(&mut self) -> Result<()> {
        self.masm.epilogue(self.frame.locals_size);
        Ok(())
    }

    fn zero_local_slots(&mut self) {
        let range = &self.frame.defined_locals_range;
        if range.0.start() == range.0.end() {
            return;
        }

        // Divide the locals range into word-size slots; first ensure that the range limits
        // are word size aligned; since there's no guarantee about their alignment. The aligned "upper"
        // limit should always be less than or equal to the size of the local area, which gets
        // validated when getting the address of a local

        let word_size = <A as ABI>::word_bytes();
        // If the locals range start is not aligned to the word size, zero the last four bytes
        let range_start = range
            .0
            .start()
            .checked_rem(word_size)
            .map_or(*range.0.start(), |_| {
                // TODO use `align_to` instead?
                let start = range.0.start() + 4;
                let addr = self.masm.local_address(&LocalSlot::i32(start));
                self.masm.store(RegImm::imm(0), addr, OperandSize::S64);
                start
            });

        // Ensure that the range end is also word-size aligned
        let range_end = align_to(*range.0.end(), word_size);
        // Divide the range into word-size slots
        let slots = (range_end - range_start) / word_size;

        match slots {
            1 => {
                let slot = LocalSlot::i64(range_start + word_size);
                let addr = self.masm.local_address(&slot);
                self.masm.store(RegImm::imm(0), addr, OperandSize::S64);
            }
            // TODO
            // Add an upper bound to this generation;
            // given a considerably large amount of slots
            // this will be inefficient
            n => {
                // Request a gpr and zero it
                let zero = self.any_gpr();
                self.masm.zero(zero);
                // store zero in each of the slots in the range
                for step in (range_start..range_end)
                    .into_iter()
                    .step_by(word_size as usize)
                {
                    let slot = LocalSlot::i64(step + word_size);
                    let addr = self.masm.local_address(&slot);
                    self.masm.store(RegImm::reg(zero), addr, OperandSize::S64);
                }
                self.regset.free_gpr(zero);
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

    /// Ensures that if the function has a return value,
    /// such value is loaded according to the defition of the
    /// ABI result; without multiple return values, this means
    /// in a single register; once multiple return values are
    /// supported this could be in the stack
    pub fn ensure_return(&mut self) {
        if self.sig.result.is_void() {
            return;
        }

        let named_reg = self.sig.result.result_reg();
        let reg = self.stack.pop_named_reg(named_reg);
        if reg.is_none() {
            let reg = self.gpr(named_reg);
            // TODO once we have the specific register
            //      check the stack to see what value is
            //      at the top; and move the value into the register
            //      if the top-of-stack value is a register
            //      free the register after the move
        }
    }

    /// Pops an i32 from the stack; first by checking if the top of the stack
    /// already contains a register; if not it requests the next available
    /// register from the register set and loads the i32 value at the stop of
    /// the stack into the selected register
    pub fn pop_i32(&mut self) -> Reg {
        let reg = self.stack.pop_reg();

        match reg {
            Some(r) => r,
            None => {
                let reg = self.any_gpr();
                // TODO temporarily only supporting i32 constants
                // This could be any other i32 stack value (e.g. a local slot)
                // if it's a register, we should mark it as free after
                // performing the move
                let val = self
                    .stack
                    .pop_i32_const()
                    .expect("i32 constant at the top of the stack");
                self.masm
                    .mov(RegImm::imm(val), RegImm::reg(reg), OperandSize::S32);
                reg
            }
        }
    }

    /// Allocate the next available general purpose register,
    /// spilling if none available
    fn any_gpr(&mut self) -> Reg {
        match self.regset.any_gpr() {
            None => {
                self.spill();
                self.regset
                    .any_gpr()
                    .expect("any allocatable general purpose register to be available")
            }
            Some(r) => r,
        }
    }

    /// Request an specific general purpose register,
    /// spilling if not available
    fn gpr(&mut self, named: Reg) -> Reg {
        match self.regset.gpr(named) {
            Some(r) => r,
            None => {
                self.spill();
                self.regset.gpr(named).expect(&format!(
                    "general purpose register {:?} to be available",
                    named
                ))
            }
        }
    }

    /// Spill locals and registers to memory
    fn spill(&mut self) {}
}
