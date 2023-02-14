use crate::{
    abi::{ABISig, ABI},
    masm::{MacroAssembler, OperandSize},
};
use anyhow::Result;
use call::FnCall;
use wasmparser::{BinaryReader, FuncValidator, ValType, ValidatorResources, VisitOperator};

mod context;
pub(crate) use context::*;
mod env;
pub use env::*;
pub mod call;

/// The code generation abstraction.
pub(crate) struct CodeGen<'a, A, M>
where
    M: MacroAssembler,
    A: ABI,
{
    /// The ABI-specific representation of the function signature, excluding results.
    sig: ABISig,

    /// The code generation context.
    pub context: CodeGenContext<'a>,

    /// The MacroAssembler.
    pub masm: &'a mut M,

    /// A reference to the function compilation environment.
    pub env: &'a dyn env::FuncEnv,

    /// A reference to the current ABI.
    pub abi: &'a A,
}

impl<'a, A, M> CodeGen<'a, A, M>
where
    M: MacroAssembler,
    A: ABI,
{
    pub fn new(
        masm: &'a mut M,
        abi: &'a A,
        context: CodeGenContext<'a>,
        env: &'a dyn FuncEnv,
        sig: ABISig,
    ) -> Self {
        Self {
            sig,
            context,
            masm,
            abi,
            env,
        }
    }

    /// Emit the function body to machine code.
    pub fn emit(
        &mut self,
        body: &mut BinaryReader<'a>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<()> {
        self.emit_start()
            .and_then(|_| self.emit_body(body, validator))
            .and_then(|_| self.emit_end())?;

        Ok(())
    }

    // TODO stack checks
    fn emit_start(&mut self) -> Result<()> {
        self.masm.prologue();
        self.masm.reserve_stack(self.context.frame.locals_size);
        Ok(())
    }

    fn emit_body(
        &mut self,
        body: &mut BinaryReader<'a>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<()> {
        self.spill_register_arguments();
        let defined_locals_range = &self.context.frame.defined_locals_range;
        self.masm.zero_mem_range(
            defined_locals_range.as_range(),
            <A as ABI>::word_bytes(),
            &mut self.context.regalloc,
        );

        while !body.eof() {
            let offset = body.original_position();
            body.visit_operator(&mut ValidateThenVisit(validator.visitor(offset), self))??;
        }
        validator.finish(body.original_position())?;
        return Ok(());

        struct ValidateThenVisit<'a, T, U>(T, &'a mut U);

        macro_rules! validate_then_visit {
            ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*) => {
                $(
                    fn $visit(&mut self $($(,$arg: $argty)*)?) -> Self::Output {
                        self.0.$visit($($($arg.clone()),*)?)?;
                        Ok(self.1.$visit($($($arg),*)?))
                    }
                )*
            };
        }

        impl<'a, T, U> VisitOperator<'a> for ValidateThenVisit<'_, T, U>
        where
            T: VisitOperator<'a, Output = wasmparser::Result<()>>,
            U: VisitOperator<'a>,
        {
            type Output = Result<U::Output>;

            wasmparser::for_each_operator!(validate_then_visit);
        }
    }

    /// Emit a direct function call.
    pub fn emit_call(&mut self, index: u32) {
        let callee = self.env.callee_from_index(index);
        if callee.import {
            // TODO: Only locally defined functions for now.
            unreachable!()
        }

        let sig = self.abi.sig(&callee.ty);
        let fncall = FnCall::new(self.abi, &sig, &mut self.context, self.masm);
        fncall.emit::<M, A>(self.masm, &mut self.context, index);
    }

    /// Emit the usual function end instruction sequence.
    fn emit_end(&mut self) -> Result<()> {
        self.handle_abi_result();
        self.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    fn spill_register_arguments(&mut self) {
        // TODO
        // Revisit this once the implicit VMContext argument is introduced;
        // when that happens the mapping between local slots and abi args
        // is not going to be symmetric.
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
                let addr = self.masm.local_address(local);
                let src = arg
                    .get_reg()
                    .expect("arg should be associated to a register");

                match &ty {
                    ValType::I32 => self.masm.store(src.into(), addr, OperandSize::S32),
                    ValType::I64 => self.masm.store(src.into(), addr, OperandSize::S64),
                    _ => panic!("Unsupported type {:?}", ty),
                }
            });
    }

    pub fn handle_abi_result(&mut self) {
        if self.sig.result.is_void() {
            return;
        }
        let named_reg = self.sig.result.result_reg();
        let reg = self
            .context
            .pop_to_reg(self.masm, Some(named_reg), OperandSize::S64);
        self.context.regalloc.free_gpr(reg);
    }
}
