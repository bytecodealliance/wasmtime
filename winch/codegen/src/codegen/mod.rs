use crate::{
    abi::{ABISig, ABI},
    masm::{MacroAssembler, OperandSize},
    stack::Val,
    CallingConvention,
};
use anyhow::Result;
use call::FnCall;
use wasmparser::{BinaryReader, FuncValidator, ValidatorResources, VisitOperator};
use wasmtime_environ::{FuncIndex, WasmFuncType, WasmType};

mod context;
pub(crate) use context::*;
mod env;
pub use env::*;
pub mod call;

/// The code generation abstraction.
pub(crate) struct CodeGen<'a, M>
where
    M: MacroAssembler,
{
    /// The ABI-specific representation of the function signature, excluding results.
    sig: ABISig,

    /// The code generation context.
    pub context: CodeGenContext<'a>,

    /// A reference to the function compilation environment.
    pub env: FuncEnv<'a, M::Ptr>,

    /// The MacroAssembler.
    pub masm: &'a mut M,
}

impl<'a, M> CodeGen<'a, M>
where
    M: MacroAssembler,
{
    pub fn new(
        masm: &'a mut M,
        context: CodeGenContext<'a>,
        env: FuncEnv<'a, M::Ptr>,
        sig: ABISig,
    ) -> Self {
        Self {
            sig,
            context,
            masm,
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
        self.masm
            .zero_mem_range(defined_locals_range.as_range(), &mut self.context.regalloc);

        // Save the vmctx pointer to its local slot in case we need to reload it
        // at any point.
        let vmctx_addr = self.masm.local_address(&self.context.frame.vmctx_slot);
        self.masm.store(
            <M::ABI as ABI>::vmctx_reg().into(),
            vmctx_addr,
            OperandSize::S64,
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
    pub fn emit_call(&mut self, index: FuncIndex) {
        let callee = self.env.callee_from_index(index);
        let (sig, callee_addr): (ABISig, Option<<M as MacroAssembler>::Address>) = if callee.import
        {
            let mut params = vec![WasmType::I64, WasmType::I64];
            params.extend_from_slice(callee.ty.params());
            let sig = WasmFuncType::new(params.into(), callee.ty.returns().into());

            let caller_vmctx = <M::ABI as ABI>::vmctx_reg();
            let callee_vmctx = self.context.any_gpr(self.masm);
            let callee_vmctx_offset = self.env.vmoffsets.vmctx_vmfunction_import_vmctx(index);
            let callee_vmctx_addr = self.masm.address_at_reg(caller_vmctx, callee_vmctx_offset);
            // FIXME Remove harcoded operand size, this will be needed
            // once 32-bit architectures are supported.
            self.masm
                .load(callee_vmctx_addr, callee_vmctx, OperandSize::S64);

            let callee_body_offset = self.env.vmoffsets.vmctx_vmfunction_import_wasm_call(index);
            let callee_addr = self.masm.address_at_reg(caller_vmctx, callee_body_offset);

            // Put the callee / caller vmctx at the start of the
            // range of the stack so that they are used as first
            // and second arguments.
            let stack = &mut self.context.stack;
            let location = stack.len() - (sig.params().len() - 2);
            stack.insert(location as usize, Val::reg(caller_vmctx));
            stack.insert(location as usize, Val::reg(callee_vmctx));
            (
                <M::ABI as ABI>::sig(&sig, &CallingConvention::Default),
                Some(callee_addr),
            )
        } else {
            (
                <M::ABI as ABI>::sig(&callee.ty, &CallingConvention::Default),
                None,
            )
        };

        let fncall = FnCall::new::<M>(&sig, &mut self.context, self.masm);
        if let Some(addr) = callee_addr {
            fncall.indirect::<M>(self.masm, &mut self.context, addr);
        } else {
            fncall.direct::<M>(self.masm, &mut self.context, index);
        }
    }

    /// Emit the usual function end instruction sequence.
    fn emit_end(&mut self) -> Result<()> {
        self.handle_abi_result();
        self.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    fn spill_register_arguments(&mut self) {
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
                    WasmType::I32 => self.masm.store(src.into(), addr, OperandSize::S32),
                    WasmType::I64 => self.masm.store(src.into(), addr, OperandSize::S64),
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
