use crate::{
    abi::{ABISig, ABI},
    masm::{MacroAssembler, OperandSize},
    stack::Val,
    CallingConvention,
};
use anyhow::Result;
use call::FnCall;
use smallvec::SmallVec;
use wasmparser::{BinaryReader, FuncValidator, Operator, ValidatorResources, VisitOperator};
use wasmtime_environ::{FuncIndex, WasmFuncType, WasmType};

mod context;
pub(crate) use context::*;
mod env;
pub use env::*;
pub mod call;
mod control;
pub(crate) use control::*;

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

    /// Stack frames for control flow.
    // NB The 64 is set arbitrarily, we can adjust it as
    // we see fit.
    pub control_frames: SmallVec<[ControlStackFrame; 64]>,
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
            control_frames: Default::default(),
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

        // Once we have emitted the epilogue and reserved stack space for the locals, we push the
        // base control flow block.
        self.control_frames
            .push(ControlStackFrame::function_body_block(
                self.sig.result,
                self.masm,
                &mut self.context,
            ));
        Ok(())
    }

    /// The following two helpers, handle else or end instructions when the
    /// compiler has entered into an unreachable code state. These instructions
    /// must be observed to determine if the reachability state should be
    /// restored.
    ///
    /// When the compiler is in an unreachable state, all the other instructions
    /// are not visited.
    pub fn handle_unreachable_else(&mut self) {
        let frame = self.control_frames.last_mut().unwrap();
        match frame {
            ControlStackFrame::If {
                reachable,
                original_sp_offset,
                original_stack_len,
                ..
            } => {
                if *reachable {
                    // We entered an unreachable state when compiling the
                    // if-then branch, but if the `if` was reachable at
                    // entry, the if-else branch will be reachable.
                    self.context.reachable = true;
                    // Reset the stack to the original length and offset.
                    Self::reset_stack(
                        &mut self.context,
                        self.masm,
                        *original_stack_len,
                        *original_sp_offset,
                    );
                    frame.bind_else(self.masm, self.context.reachable);
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn handle_unreachable_end(&mut self) {
        let frame = self.control_frames.pop().unwrap();
        // We just popped the outermost block.
        let is_outermost = self.control_frames.len() == 0;
        if frame.is_next_sequence_reachable() {
            self.context.reachable = true;

            let (value_stack_len, sp_offset) = frame.original_stack_len_and_sp_offset();
            // Reset the stack to the original length and offset.
            Self::reset_stack(&mut self.context, self.masm, value_stack_len, sp_offset);
            // If the current frame is the outermost frame, which corresponds to the
            // current function's body, only bind the exit label as we don't need to
            // push any more values to the value stack, else perform the entire `bind_end`
            // process, which involves pushing results to the value stack.
            if is_outermost {
                frame.bind_exit_label(self.masm);
            } else {
                frame.bind_end(self.masm, &mut self.context);
            }
        }
    }

    /// Helper function to reset value and stack pointer to the given length and stack pointer
    /// offset respectively. This function is only used when restoring the code generation's
    /// reachabiliy state when handling an unreachable `end` or `else`.
    fn reset_stack(context: &mut CodeGenContext, masm: &mut M, stack_len: usize, sp_offset: u32) {
        masm.reset_stack_pointer(sp_offset);
        context.drop_last(context.stack.len() - stack_len);
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
                        // Only visit operators if the compiler is in a reachable code state. If
                        // the compiler is in an unrechable code state, most of the operators are
                        // ignored except for If, Block, Loop, Else and End. These operators need
                        // to be observed in order to keep the control stack frames balanced and to
                        // determine if reachability should be restored.
                        let visit_when_unreachable = visit_op_when_unreachable(Operator::$op $({ $($arg: $arg.clone()),* })?);
                        if self.1.is_reachable() || visit_when_unreachable  {
                            Ok(self.1.$visit($($($arg),*)?))
                        } else {
                            Ok(U::Output::default())
                        }
                    }
                )*
            };
        }

        fn visit_op_when_unreachable(op: Operator) -> bool {
            use Operator::*;
            match op {
                If { .. } | Block { .. } | Loop { .. } | Else | End => true,
                _ => false,
            }
        }

        /// Trait to handle reachability state.
        trait ReachableState {
            /// Returns true if the current state of the program is reachable.
            fn is_reachable(&self) -> bool;
        }

        impl<'a, M: MacroAssembler> ReachableState for CodeGen<'a, M> {
            fn is_reachable(&self) -> bool {
                self.context.reachable
            }
        }

        impl<'a, T, U> VisitOperator<'a> for ValidateThenVisit<'_, T, U>
        where
            T: VisitOperator<'a, Output = wasmparser::Result<()>>,
            U: VisitOperator<'a> + ReachableState,
            U::Output: Default,
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
        assert!(self.context.stack.len() == 0);
        self.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    /// Returns the control stack frame at the given depth.
    ///
    /// # Panics
    /// This function panics if the given depth cannot be associated
    /// with a control stack frame.
    pub fn control_at(frames: &mut [ControlStackFrame], depth: u32) -> &mut ControlStackFrame {
        let index = (frames.len() - 1)
            .checked_sub(depth as usize)
            .unwrap_or_else(|| panic!("expected valid control stack frame at index: {}", depth));

        &mut frames[index]
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
}
