use crate::{
    abi::{ABISig, ABI},
    isa::reg::Reg,
    masm::RegImm,
    masm::{CmpKind, MacroAssembler, OperandSize, TrapCode},
    stack::{TypedReg, Val},
    CallingConvention,
};
use anyhow::Result;
use smallvec::SmallVec;
use wasmparser::{BinaryReader, FuncValidator, Operator, ValidatorResources, VisitOperator};
use wasmtime_environ::{
    PtrSize, TableIndex, TypeIndex, WasmFuncType, WasmHeapType, WasmType, FUNCREF_MASK,
};

mod context;
pub(crate) use context::*;
mod env;
pub use env::*;
mod call;
pub(crate) use call::*;
mod control;
pub(crate) use control::*;
mod builtin;
pub(crate) use builtin::*;

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
                original_stack_len,
                ..
            } => {
                if *reachable {
                    // We entered an unreachable state when compiling the
                    // if-then branch, but if the `if` was reachable at
                    // entry, the if-else branch will be reachable.
                    self.context.reachable = true;
                    // Reset the stack to the original length and offset.
                    Self::reset_stack(&mut self.context, *original_stack_len);
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

            let (value_stack_len, _) = frame.original_stack_len_and_sp_offset();
            // Reset the stack to the original length and offset.
            Self::reset_stack(&mut self.context, value_stack_len);
            // If the current frame is the outermost frame, which corresponds to the
            // current function's body, only bind the exit label as we don't need to
            // push any more values to the value stack, else perform the entire `bind_end`
            // process, which involves pushing results to the value stack.
            if is_outermost {
                frame.bind_exit_label(self.masm);
            } else {
                frame.bind_end(self.masm, &mut self.context);
            }
        } else if is_outermost {
            // If we reach the end of the function in an unreachable
            // state, perform the necessary cleanup to leave the stack
            // and SP in the expected state.  The compiler can enter
            // in this state through an infinite loop.
            let (value_stack_len, target_sp) = frame.original_stack_len_and_sp_offset();
            Self::reset_stack(&mut self.context, value_stack_len);
            if self.masm.sp_offset() > target_sp {
                self.masm.free_stack(self.masm.sp_offset() - target_sp);
            }
        }
    }

    /// Helper function to reset value and stack pointer to the given length and stack pointer
    /// offset respectively. This function is only used when restoring the code generation's
    /// reachabiliy state when handling an unreachable `end` or `else`.
    pub fn reset_stack(context: &mut CodeGenContext, target_stack_len: usize) {
        // `CodeGenContext::reset_stack` only gets called when
        // handling unreachable end or unreachable else, so we only
        // care about freeing any registers in the provided range.
        context.drop_last(
            context.stack.len() - target_stack_len,
            |regalloc, val| match val {
                Val::Reg(tr) => regalloc.free(tr.reg),
                _ => {}
            },
        );
    }

    fn emit_body(
        &mut self,
        body: &mut BinaryReader<'a>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<()> {
        self.spill_register_arguments();
        let defined_locals_range = &self.context.frame.defined_locals_range;
        self.masm.zero_mem_range(defined_locals_range.as_range());

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

    /// Emit a function call to:
    /// * A locally defined function.
    /// * A function import.
    /// * A funcref.
    pub fn emit_call(&mut self, callee: Callee) {
        let ptr_type = self.env.ptr_type();
        match callee {
            Callee::Import(callee) => {
                let mut params = Vec::with_capacity(callee.ty.params().len() + 2);
                params.extend_from_slice(&self.env.vmctx_args_type());
                params.extend_from_slice(callee.ty.params());
                let sig = WasmFuncType::new(params.into(), callee.ty.returns().into());

                let caller_vmctx = <M::ABI as ABI>::vmctx_reg();
                let callee_vmctx = self.context.any_gpr(self.masm);
                let callee_vmctx_offset = self
                    .env
                    .vmoffsets
                    .vmctx_vmfunction_import_vmctx(callee.index);
                let callee_vmctx_addr = self.masm.address_at_vmctx(callee_vmctx_offset);
                self.masm.load_ptr(callee_vmctx_addr, callee_vmctx);

                let callee_body_offset = self
                    .env
                    .vmoffsets
                    .vmctx_vmfunction_import_wasm_call(callee.index);
                let callee_addr = self.masm.address_at_vmctx(callee_body_offset);

                // Put the callee / caller vmctx at the start of the
                // range of the stack so that they are used as first
                // and second arguments.
                let stack = &mut self.context.stack;
                let location = stack.len() - (sig.params().len() - 2);
                let values = [
                    TypedReg::new(ptr_type, callee_vmctx).into(),
                    TypedReg::new(ptr_type, caller_vmctx).into(),
                ]
                .into_iter();
                self.context.stack.insert_many(location, values);

                let abi_sig = <M::ABI as ABI>::sig(&sig, &CallingConvention::Default);
                FnCall::new(&abi_sig)
                    .save_live_registers(&mut self.context, self.masm)
                    .addr(self.masm, &mut self.context, callee_addr);
            }

            Callee::Local(callee) => {
                let abi_sig = <M::ABI as ABI>::sig(&callee.ty, &CallingConvention::Default);
                FnCall::new(&abi_sig)
                    .save_live_registers(&mut self.context, self.masm)
                    .direct(self.masm, &mut self.context, callee.index);
            }

            Callee::FuncRef(ty) => {
                // Get type for the caller and callee VMContext.
                let abi_sig = <M::ABI as ABI>::sig(&ty, &CallingConvention::Default);
                // Pop the funcref pointer to a register and allocate a register to hold the
                // address of the funcref. Since the callee is not addressed from a global non
                // allocatable register (like the vmctx in the case of an import), we load the
                // funcref to a register ensuring that it doesn't get assigned to a non-arg
                // register.
                let (funcref_ptr, funcref) = self.context.without::<_, M, _>(
                    abi_sig.param_regs(),
                    abi_sig.param_regs(),
                    self.masm,
                    |cx, masm| (cx.pop_to_reg(masm, None).into(), cx.any_gpr(masm)),
                );
                self.masm.load(
                    self.masm.address_at_reg(
                        funcref_ptr,
                        self.env.vmoffsets.ptr.vm_func_ref_wasm_call().into(),
                    ),
                    funcref,
                    ptr_type.into(),
                );
                self.context.free_reg(funcref_ptr);

                FnCall::new(&abi_sig)
                    .save_live_registers(&mut self.context, self.masm)
                    .reg(self.masm, &mut self.context, funcref);
            }
        };
    }

    /// Emits a a series of instructions that will type check a function reference call.
    pub fn emit_typecheck_funcref(&mut self, funcref_ptr: Reg, type_index: TypeIndex) {
        let ptr_size: OperandSize = self.env.ptr_type().into();
        let sig_index_bytes = self.env.vmoffsets.size_of_vmshared_signature_index();
        let sig_size = OperandSize::from_bytes(sig_index_bytes);
        let sig_index = self.env.translation.module.types[type_index].unwrap_function();
        let sig_offset = sig_index
            .as_u32()
            .checked_mul(sig_index_bytes.into())
            .unwrap();
        let signatures_base_offset = self.env.vmoffsets.vmctx_signature_ids_array();
        let scratch = <M::ABI as ABI>::scratch_reg();
        let funcref_sig_offset = self.env.vmoffsets.ptr.vm_func_ref_type_index();

        // Load the signatures address into the scratch register.
        self.masm.load(
            self.masm.address_at_vmctx(signatures_base_offset),
            scratch,
            ptr_size,
        );

        // Get the caller id.
        let caller_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm.address_at_reg(scratch, sig_offset),
            caller_id,
            sig_size,
        );

        let callee_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm
                .address_at_reg(funcref_ptr, funcref_sig_offset.into()),
            callee_id,
            sig_size,
        );

        // Typecheck.
        self.masm.cmp(callee_id.into(), caller_id, OperandSize::S32);
        self.masm.trapif(CmpKind::Ne, TrapCode::BadSignature);
        self.context.free_reg(callee_id);
        self.context.free_reg(caller_id);
    }

    /// Emit the usual function end instruction sequence.
    fn emit_end(&mut self) -> Result<()> {
        assert!(self.context.stack.len() == 0);
        self.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    fn spill_register_arguments(&mut self) {
        use WasmType::*;
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
                    I32 | I64 | F32 | F64 => self.masm.store(src.into(), addr, ty.into()),
                    Ref(rt) => match rt.heap_type {
                        WasmHeapType::Func => self.masm.store_ptr(src.into(), addr),
                        ht => unimplemented!("Support for WasmHeapType: {ht}"),
                    },
                    _ => unimplemented!("Support for WasmType {ty}"),
                }
            });
    }

    /// Emits a series of instructions to lazily initialize a function reference.
    pub fn emit_lazy_init_funcref(
        table_data: &TableData,
        table_index: TableIndex,
        ptr_type: WasmType,
        context: &mut CodeGenContext,
        masm: &mut M,
        call: &mut FnCall,
        callee: Reg,
    ) {
        let index = context.pop_to_reg(masm, None);
        let elem_value: Reg = context.any_gpr(masm).into();
        let base = context.any_gpr(masm);
        let elem_addr = masm.table_elem_address(index.into(), base, &table_data, context);
        masm.load_ptr(elem_addr, elem_value);

        let defined = masm.get_label();
        let cont = masm.get_label();

        // Preemptively move the table element address to the
        // result register, to avoid conflicts at the control flow merge.
        let result = call.abi_sig.result.result_reg().unwrap();
        masm.mov(elem_value.into(), result, ptr_type.into());

        // Push the builtin function arguments to the stack.
        context
            .stack
            .push(TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg()).into());
        context.stack.push(table_index.as_u32().try_into().unwrap());
        context.stack.push(index.into());

        // `branch` in this case will perform a test of the given register,
        // and jump to the defined branch if it's not zero.
        masm.branch(
            CmpKind::Ne,
            elem_value.into(),
            elem_value,
            defined,
            ptr_type.into(),
        );

        call.calculate_call_stack_space(context)
            .reg(masm, context, callee);
        // We know the signature of the libcall in this case, so we assert that there's
        // one element in the stack and that it's  the ABI signature's result register.
        let top = context.stack.peek().unwrap();
        let top = top.get_reg();
        debug_assert!(top.reg == result);
        masm.jmp(cont);

        // In the defined case, mask the funcref address in place, by peeking into the
        // last element of the value stack, which was pushed by the `indirect` function
        // call above.
        masm.bind(defined);
        let imm = RegImm::i64(FUNCREF_MASK as i64);
        let dst = top.into();
        masm.and(dst, dst, imm, top.ty.into());

        masm.bind(cont);
        // The indirect call above, will take care of freeing the registers used as
        // params.
        // So we only free the params used to lazily initialize the func ref.
        context.free_reg(base);
        context.free_reg(elem_value);
    }
}

/// Returns the index of the [`ControlStackFrame`] for the given
/// depth.
pub fn control_index(depth: u32, control_length: usize) -> usize {
    (control_length - 1)
        .checked_sub(depth as usize)
        .unwrap_or_else(|| panic!("expected valid control stack frame at index: {}", depth))
}
