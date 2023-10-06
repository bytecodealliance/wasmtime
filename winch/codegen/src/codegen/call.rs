//! Function call emission.  For more details around the ABI and
//! calling convention, see [ABI].
use crate::{
    abi::{ABIArg, ABIResult, ABISig, ABI},
    codegen::{BuiltinFunction, CodeGenContext},
    masm::{CalleeKind, MacroAssembler, OperandSize},
    reg::Reg,
};
use wasmtime_environ::FuncIndex;

/// All the information needed to emit a function call.
#[derive(Copy, Clone)]
pub(crate) struct FnCall<'a> {
    /// The stack space consumed by the function call; that is,
    /// the sum of:
    ///
    /// 1. The amount of stack space created by saving any live
    ///    registers at the callsite.
    /// 2. The amount of space used by any memory entries in the value
    ///    stack present at the callsite, that will be used as
    ///    arguments for the function call. Any memory values in the
    ///    value stack that are needed as part of the function
    ///    arguments, will be consumed by the function call (either by
    ///    assigning those values to a register or by storing those
    ///    values to a memory location if the callee argument is on
    ///    the stack), so we track that stack space to reclaim it once
    ///    the function call has ended. This could also be done in
    ///    `assign_args` everytime a memory entry needs to be assigned
    ///    to a particular location, but doing so, will incur in more
    ///    instructions (e.g. a pop per argument that needs to be
    ///    assigned); it's more efficient to track the space needed by
    ///    those memory values and reclaim it at once.
    ///
    /// The machine stack throghout the function call is as follows:
    /// ┌──────────────────────────────────────────────────┐
    /// │                                                  │
    /// │                  1                               │
    /// │  Stack space created by any previous spills      │
    /// │  from the value stack; and which memory values   │
    /// │  are used as function arguments.                 │
    /// │                                                  │
    /// ├──────────────────────────────────────────────────┤ ---> The Wasm value stack at this point in time would look like:
    /// │                                                  │      [ Reg | Reg | Mem(offset) | Mem(offset) ]
    /// │                   2                              │
    /// │   Stack space created by saving                  │
    /// │   any live registers at the callsite.            │
    /// │                                                  │
    /// │                                                  │
    /// ├─────────────────────────────────────────────────┬┤ ---> The Wasm value stack at this point in time would look like:
    /// │                                                  │      [ Mem(offset) | Mem(offset) | Mem(offset) | Mem(offset) ]
    /// │                                                  │      Assuming that the callee takes 4 arguments, we calculate
    /// │                                                  │      2 spilled registers + 2 memory values; all of which will be used
    /// │   Stack space allocated for                      │      as arguments to the call via `assign_args`, thus the memory they represent is
    /// │   the callee function arguments in the stack;    │      is considered to be consumed by the call.
    /// │   represented by `arg_stack_space`               │
    /// │                                                  │
    /// │                                                  │
    /// │                                                  │
    /// └──────────────────────────────────────────────────┘ ------> Stack pointer when emitting the call
    ///
    call_stack_space: Option<u32>,
    /// The total stack space needed for the callee arguments on the
    /// stack, including any adjustments to the function's frame and
    /// aligned to to the required ABI alignment.
    arg_stack_space: u32,
    /// The ABI-specific signature of the callee.
    pub abi_sig: &'a ABISig,
    /// Whether this a built-in function call.
    lib: bool,
}

impl<'a> FnCall<'a> {
    /// Creates a new [`FnCall`] from the callee's [`ABISig`].
    pub fn new(callee_sig: &'a ABISig) -> Self {
        Self {
            abi_sig: &callee_sig,
            arg_stack_space: callee_sig.stack_bytes,
            call_stack_space: None,
            lib: false,
        }
    }

    /// Saves any live registers and records the stack space that will be
    /// consumed by the function call. The stack space consumed by the call must
    /// be known before emitting the call via any of the emission variants:
    /// [`FnCall::direct`], [`FnCall::indirect`] or [`FnCall::addr`], which
    /// means that the call stack space must be calculated either by invoking
    /// [`FnCall::save_live_registers`] or
    /// [`FnCall::calculate_call_stack_space`] before invoking any of
    /// the emission variants.
    pub fn save_live_registers<M: MacroAssembler>(
        &mut self,
        context: &mut CodeGenContext,
        masm: &mut M,
    ) -> &mut Self {
        // Invariant: ensure that `call_stack_space` is only set once: either by
        // [`FnCall::save_live_registers`] or
        // [`FnCall::calculate_call_stack_space`]
        debug_assert!(self.call_stack_space.is_none());
        let callee_params = &self.abi_sig.params;
        let stack = &context.stack;
        let call_stack_space = match callee_params.len() {
            0 => {
                let _ = context.save_live_registers_and_calculate_sizeof(masm, ..);
                0u32
            }
            _ => {
                // Here we perform a "spill" of the register entries
                // in the Wasm value stack, we also count any memory
                // values that will be used used as part of the callee
                // arguments.  Saving the live registers is done by
                // emitting push operations for every `Reg` entry in
                // the Wasm value stack. We do this to be compliant
                // with Winch's internal ABI, in which all registers
                // are treated as caller-saved. For more details, see
                // [ABI].
                //
                // The next few lines, partition the value stack into
                // two sections:
                // +------------------+--+--- (Stack top)
                // |                  |  |
                // |                  |  | 1. The top `n` elements, which are used for
                // |                  |  |    function arguments; for which we save any
                // |                  |  |    live registers, keeping track of the amount of registers
                // +------------------+  |    saved plus the amount of memory values consumed by the function call;
                // |                  |  |    with this information we can later reclaim the space used by the function call.
                // |                  |  |
                // +------------------+--+---
                // |                  |  | 2. The rest of the items in the stack, for which
                // |                  |  |    we only save any live registers.
                // |                  |  |
                // +------------------+  |
                assert!(stack.len() >= callee_params.len());
                let partition = stack.len() - callee_params.len();
                let _ = context.save_live_registers_and_calculate_sizeof(masm, 0..partition);
                context.save_live_registers_and_calculate_sizeof(masm, partition..)
            }
        };

        self.call_stack_space = Some(call_stack_space);
        self
    }

    /// Records the stack space that will be needeed by the function call by
    /// scanning the value stack and returning the size of the all the memory
    /// entries present in callee's argument length range.  The stack space
    /// consumed by the call must be known before emitting the call via any of
    /// the emission variants: [`FnCall::direct`], [`FnCall::indirect`] or
    /// [`FnCall::addr`], which means that the call stack space must be
    /// calculated either by invoking [`FnCall::save_live_registers`] or
    /// [`FnCall::calculate_call_stack_space`] before invoking any of
    /// the emission variants.
    /// This function is particularly useful when there's no need to save any
    /// live registers before emitting the function call. This could happen when
    /// emitting calls to libcalls: [`FnCall::with_lib`] will eagerly save all
    /// the live registers when invoked and will also ensure that any registers
    /// allocated after are non argument registers, in which case if any of
    /// those registers need to go on the value stack to be used as function
    /// arguments, they don't need to be saved.
    pub fn calculate_call_stack_space(&mut self, context: &mut CodeGenContext) -> &mut Self {
        // Invariant: ensure that `call_stack_space` is only set once: either by
        // [`FnCall::save_live_registers`] or
        // [`FnCall::calculate_call_stack_space`]
        debug_assert!(self.call_stack_space.is_none());
        let params_len = self.abi_sig.params.len();
        assert!(context.stack.len() >= params_len);

        let stack_len = context.stack.len();
        let call_stack_space = if params_len == 0 {
            0
        } else {
            context.stack.sizeof((stack_len - params_len)..)
        };
        self.call_stack_space = Some(call_stack_space);
        self
    }

    /// Emit a direct function call, to a locally defined function.
    pub fn direct<M: MacroAssembler>(
        self,
        masm: &mut M,
        context: &mut CodeGenContext,
        callee: FuncIndex,
    ) {
        // Invariant: `call_stack_space` must be known.
        debug_assert!(self.call_stack_space.is_some());
        let reserved_stack = masm.call(self.arg_stack_space, |masm| {
            self.assign_args(context, masm, <M::ABI as ABI>::scratch_reg());
            CalleeKind::direct(callee.as_u32())
        });
        self.post_call::<M>(masm, context, reserved_stack);
    }

    /// Emit an indirect function call, using a register.
    pub fn reg<M: MacroAssembler>(self, masm: &mut M, context: &mut CodeGenContext, reg: Reg) {
        // Invariant: `call_stack_space` must be known.
        debug_assert!(self.call_stack_space.is_some());
        let reserved_stack = masm.call(self.arg_stack_space, |masm| {
            let scratch = <M::ABI as ABI>::scratch_reg();
            self.assign_args(context, masm, scratch);
            CalleeKind::indirect(reg)
        });
        context.free_reg(reg);
        self.post_call::<M>(masm, context, reserved_stack);
    }

    /// Emit an indirect function call, using a an address.
    /// This function will load the provided address into a unallocatable
    /// scratch register.
    pub fn addr<M: MacroAssembler>(
        self,
        masm: &mut M,
        context: &mut CodeGenContext,
        callee: M::Address,
    ) {
        // Invariant: `call_stack_space` must be known.
        debug_assert!(self.call_stack_space.is_some());
        let reserved_stack = masm.call(self.arg_stack_space, |masm| {
            let scratch = <M::ABI as ABI>::scratch_reg();
            self.assign_args(context, masm, scratch);
            masm.load(callee, scratch, OperandSize::S64);
            CalleeKind::indirect(scratch)
        });

        self.post_call::<M>(masm, context, reserved_stack);
    }

    /// Prepares the compiler to call a built-in function (libcall).
    /// This fuction, saves all the live registers and loads the callee
    /// address into a non-argument register which is then passed to the
    /// caller through the provided callback.
    ///
    /// It is the caller's responsibility to finalize the function call
    /// by calling `FnCall::reg` once all the information is known.
    pub fn with_lib<M: MacroAssembler, F>(
        &mut self,
        masm: &mut M,
        context: &mut CodeGenContext,
        func: &BuiltinFunction,
        mut f: F,
    ) where
        F: FnMut(&mut CodeGenContext, &mut M, &mut Self, Reg),
    {
        self.lib = true;
        // When dealing with libcalls, we don't have all the information
        // upfront (all necessary arguments in the stack) in order to optimize
        // saving the live registers, so we save all the values available in
        // the value stack.
        context.spill(masm);
        let vmctx = <M::ABI as ABI>::vmctx_reg();
        let scratch = <M::ABI as ABI>::scratch_reg();

        let builtins_base = masm.address_at_reg(vmctx, func.base);
        masm.load(builtins_base, scratch, OperandSize::S64);
        let builtin_func_addr = masm.address_at_reg(scratch, func.offset);
        context.without::<(), M, _>(
            // Do not free the result registers if any as the function call will
            // push them onto the stack as a result of the call.
            self.abi_sig.regs(),
            self.abi_sig.param_regs(),
            masm,
            |cx, masm| {
                let callee = cx.any_gpr(masm);
                masm.load_ptr(builtin_func_addr, callee);
                f(cx, masm, self, callee);
                cx.free_reg(callee);
            },
        );
    }

    fn post_call<M: MacroAssembler>(&self, masm: &mut M, context: &mut CodeGenContext, size: u32) {
        masm.free_stack(self.call_stack_space.unwrap() + size);
        // Only account for registers given that any memory entries
        // consumed by the call (assigned to a register or to a stack
        // slot) were freed by the previous call to
        // `masm.free_stack`, so we only care about dropping them
        // here.
        //
        // NOTE / TODO there's probably a path to getting rid of
        // `save_live_registers_and_calculate_sizeof` and
        // `call_stack_space`, making it a bit more obvious what's
        // happening here. We could:
        //
        // * Modify the `spill` implementation so that it takes a
        // filtering callback, to control which values the caller is
        // interested in saving (e.g. save all if no function is provided)
        // * Rely on the new implementation of `drop_last` to calcuate
        // the stack memory entries consumed by the call and then free
        // the calculated stack space.
        context.drop_last(self.abi_sig.params.len(), |regalloc, v| {
            if v.is_reg() {
                regalloc.free(v.get_reg().into());
            }
        });

        // When emitting built-calls we ensure that none of the registers
        // (params and results) used as part of the ABI signature are
        // allocatable throughout the lifetime of the `with_lib` callback, since
        // such registers will be used to assign arguments and hold results.
        // After executing the callback, it's only safe to free the param
        // registers, since depending on the signature, the caller
        // will push any result registers to the stack, keeping those registers allocated.
        // Here we ensure that any allocated result registers are correctly
        // freed before finalizing the function call and pushing any results to
        // the value stack.
        if self.lib {
            match self.abi_sig.result {
                ABIResult::Reg { reg, .. } => {
                    assert!(!context.regalloc.reg_available(reg));
                    context.free_reg(reg);
                }
                _ => {}
            }
        }
        context.push_abi_results(&self.abi_sig.result, masm);
    }

    fn assign_args<M: MacroAssembler>(
        &self,
        context: &mut CodeGenContext,
        masm: &mut M,
        scratch: Reg,
    ) {
        let arg_count = self.abi_sig.params.len();
        let stack = &context.stack;
        let mut stack_values = stack.peekn(arg_count);
        for arg in &self.abi_sig.params {
            let val = stack_values
                .next()
                .unwrap_or_else(|| panic!("expected stack value for function argument"));
            match &arg {
                &ABIArg::Reg { ty: _, reg } => {
                    context.move_val_to_reg(&val, *reg, masm);
                }
                &ABIArg::Stack { ty, offset } => {
                    let addr = masm.address_at_sp(*offset);
                    let size: OperandSize = (*ty).into();
                    context.move_val_to_reg(val, scratch, masm);
                    masm.store(scratch.into(), addr, size);
                }
            }
        }
    }
}
