use crate::TRAP_INTERNAL_ASSERT;
use crate::compiler::Compiler;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types::I8;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::{BuiltinFunctionIndex, TripleExt};

/// Helper trait to share translation of traps between core functions and
/// component trampolines.
///
/// Traps are conditionally performed as libcalls when signals-based-traps are
/// disabled, for example, but otherwise use the native CLIF `trap` instruction.
pub trait TranslateTrap {
    fn compiler(&self) -> &Compiler;
    fn vmctx_val(&mut self, cursor: &mut FuncCursor<'_>) -> ir::Value;
    fn builtin_funcref(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        index: BuiltinFunctionIndex,
    ) -> ir::FuncRef;

    fn trap(&mut self, builder: &mut FunctionBuilder, trap: ir::TrapCode) {
        match (
            self.clif_instruction_traps_enabled(),
            crate::clif_trap_to_env_trap(trap),
        ) {
            // If libcall traps are disabled or there's no wasmtime-defined trap
            // code for this, then emit a native trap instruction.
            (true, _) | (_, None) => {
                builder.ins().trap(trap);
            }
            // ... otherwise with libcall traps explicitly enabled and a
            // wasmtime-based trap code invoke the libcall to raise a trap and
            // pass in our trap code. Leave a debug `unreachable` in place
            // afterwards as a defense-in-depth measure.
            (false, Some(trap)) => {
                let trap_libcall = self.builtin_funcref(builder, BuiltinFunctionIndex::trap());
                let vmctx = self.vmctx_val(&mut builder.cursor());
                let trap_code = builder.ins().iconst(I8, i64::from(trap as u8));
                builder.ins().call(trap_libcall, &[vmctx, trap_code]);
                let raise_libcall = self.builtin_funcref(builder, BuiltinFunctionIndex::raise());
                builder.ins().call(raise_libcall, &[vmctx]);
                builder.ins().trap(TRAP_INTERNAL_ASSERT);
            }
        }
    }

    fn trapz(&mut self, builder: &mut FunctionBuilder, value: ir::Value, trap: ir::TrapCode) {
        if self.clif_instruction_traps_enabled() {
            builder.ins().trapz(value, trap);
        } else {
            let ty = builder.func.dfg.value_type(value);
            let zero = builder.ins().iconst(ty, 0);
            let cmp = builder.ins().icmp(IntCC::Equal, value, zero);
            self.conditionally_trap(builder, cmp, trap);
        }
    }

    fn trapnz(&mut self, builder: &mut FunctionBuilder, value: ir::Value, trap: ir::TrapCode) {
        if self.clif_instruction_traps_enabled() {
            builder.ins().trapnz(value, trap);
        } else {
            let ty = builder.func.dfg.value_type(value);
            let zero = builder.ins().iconst(ty, 0);
            let cmp = builder.ins().icmp(IntCC::NotEqual, value, zero);
            self.conditionally_trap(builder, cmp, trap);
        }
    }

    fn uadd_overflow_trap(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
        trap: ir::TrapCode,
    ) -> ir::Value {
        if self.clif_instruction_traps_enabled() {
            builder.ins().uadd_overflow_trap(lhs, rhs, trap)
        } else {
            let (ret, overflow) = builder.ins().uadd_overflow(lhs, rhs);
            self.conditionally_trap(builder, overflow, trap);
            ret
        }
    }

    /// Helper to emit a conditional trap based on `trap_cond`.
    ///
    /// This should only be used if `self.clif_instruction_traps_enabled()` is
    /// false, otherwise native CLIF instructions should be used instead.
    fn conditionally_trap(
        &mut self,
        builder: &mut FunctionBuilder,
        trap_cond: ir::Value,
        trap: ir::TrapCode,
    ) {
        assert!(!self.clif_instruction_traps_enabled());

        let trap_block = builder.create_block();
        builder.set_cold_block(trap_block);
        let continuation_block = builder.create_block();

        builder
            .ins()
            .brif(trap_cond, trap_block, &[], continuation_block, &[]);

        builder.seal_block(trap_block);
        builder.seal_block(continuation_block);

        builder.switch_to_block(trap_block);
        self.trap(builder, trap);
        builder.switch_to_block(continuation_block);
    }

    /// Returns whether it's acceptable to have CLIF instructions natively trap,
    /// such as division-by-zero.
    ///
    /// This is enabled if `signals_based_traps` is `true` or on
    /// Pulley unconditionally since Pulley doesn't use hardware-based
    /// traps in its runtime. However, if guest debugging is enabled,
    /// then we cannot rely on Pulley traps and still need a libcall
    /// to gain proper ownership of the store in the runtime's
    /// debugger hooks.
    fn clif_instruction_traps_enabled(&self) -> bool {
        let tunables = self.compiler().tunables();
        tunables.signals_based_traps || (self.is_pulley() && !tunables.debug_guest)
    }

    /// Returns whether translation is happening for Pulley bytecode.
    fn is_pulley(&self) -> bool {
        self.compiler().isa().triple().is_pulley()
    }
}
