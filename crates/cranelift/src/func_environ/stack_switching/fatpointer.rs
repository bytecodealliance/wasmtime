use cranelift_codegen::ir;
use cranelift_codegen::ir::InstBuilder;

/// The Cranelift type used to represent all of the following:
/// - wasm values of type `(ref null $ct)` and `(ref $ct)`
/// - equivalently: runtime values of type `Option<VMContObj>` and `VMContObj`
/// Note that a `VMContObj` is a fat pointer
/// consisting of a pointer to `VMContRef` and a 64 bit sequence
/// counter.
/// We represent this here as a 128bit value, with the same representation as
/// `core::mem::transmute::<i128, VMContObj>`.
pub const POINTER_TYPE: ir::Type = ir::types::I128;

/// Turns a (possibly null) reference to a continuation object into a tuple
/// (revision, contref_ptr). If `contobj` denotes a wasm null reference, the
/// contref_ptr part will be a null pointer.
pub(crate) fn deconstruct<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    pos: &mut cranelift_codegen::cursor::FuncCursor,
    contobj: ir::Value,
) -> (ir::Value, ir::Value) {
    debug_assert_eq!(pos.func.dfg.value_type(contobj), POINTER_TYPE);
    let ptr_ty = env.pointer_type();
    assert!(ptr_ty.bits() <= 64);

    let contref = pos.ins().ireduce(ptr_ty, contobj);
    let shifted = pos.ins().ushr_imm(contobj, 64);
    let revision_counter = pos.ins().ireduce(ir::types::I64, shifted);

    (revision_counter, contref)
}

/// Constructs a continuation object from a given contref and revision pointer.
/// The contref_addr may be 0, to indicate that we want to build a wasm null reference.
pub(crate) fn construct<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    pos: &mut cranelift_codegen::cursor::FuncCursor,
    revision_counter: ir::Value,
    contref_addr: ir::Value,
) -> ir::Value {
    debug_assert_eq!(pos.func.dfg.value_type(contref_addr), env.pointer_type());
    debug_assert_eq!(pos.func.dfg.value_type(revision_counter), ir::types::I64);
    assert!(env.pointer_type().bits() <= 64);

    let contref_addr = pos.ins().uextend(ir::types::I128, contref_addr);
    let revision_counter = pos.ins().uextend(ir::types::I128, revision_counter);
    let shifted_counter = pos.ins().ishl_imm(revision_counter, 64);
    let contobj = pos.ins().bor(shifted_counter, contref_addr);

    contobj
}
