use cranelift_codegen::ir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::types::{I32, I64};
use cranelift_frontend::FunctionBuilder;

/// Universal control effect. This structure encodes return signal,
/// resume signal, suspension signal, and handler index into a
/// u64 value. This instance is used at compile time. There is a runtime
/// counterpart in `continuations/src/lib.rs`.
/// We convert to and from u64 as follows: The low 32 bits of the u64 are the
/// discriminant, the high 32 bits are the handler_index (if `Suspend`)
#[derive(Clone, Copy)]
pub struct ControlEffect(ir::Value);

impl ControlEffect {
    // Returns the discriminant
    pub fn signal(&self, builder: &mut FunctionBuilder) -> ir::Value {
        builder.ins().ushr_imm(self.0, 32)
    }

    pub fn from_u64(val: ir::Value) -> Self {
        Self(val)
    }

    pub fn to_u64(&self) -> ir::Value {
        self.0
    }

    pub fn encode_resume(builder: &mut FunctionBuilder) -> Self {
        let discriminant = builder.ins().iconst(
            I64,
            i64::from(wasmtime_environ::CONTROL_EFFECT_RESUME_DISCRIMINANT),
        );
        let val = builder.ins().ishl_imm(discriminant, 32);

        Self(val)
    }

    pub fn encode_switch(builder: &mut FunctionBuilder) -> Self {
        let discriminant = builder.ins().iconst(
            I64,
            i64::from(wasmtime_environ::CONTROL_EFFECT_SWITCH_DISCRIMINANT),
        );
        let val = builder.ins().ishl_imm(discriminant, 32);

        Self(val)
    }

    pub fn encode_suspend(builder: &mut FunctionBuilder, handler_index: ir::Value) -> Self {
        let discriminant = builder.ins().iconst(
            I64,
            i64::from(wasmtime_environ::CONTROL_EFFECT_SUSPEND_DISCRIMINANT),
        );
        let val = builder.ins().ishl_imm(discriminant, 32);
        let handler_index = builder.ins().uextend(I64, handler_index);
        let val = builder.ins().bor(val, handler_index);

        Self(val)
    }

    /// Returns the payload of the `Suspend` variant
    pub fn handler_index(self, builder: &mut FunctionBuilder) -> ir::Value {
        builder.ins().ireduce(I32, self.0)
    }
}
