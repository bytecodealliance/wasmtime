use cranelift_codegen::ir;
use cranelift_codegen::ir::InstBuilder;

/// Returns the Cranelift type used to represent all of the following:
/// - wasm values of type `(ref null $ct)` and `(ref $ct)`
/// - equivalently: runtime values of type `Option<VMContObj>` and `VMContObj`
/// Note that a `VMContObj` is a fat pointer consisting of a pointer to
/// `VMContRef` and a pointer-sized revision counter. We represent this as 2 words
/// (pointer and usize).
pub fn fatpointer_type(env: &crate::func_environ::FuncEnvironment) -> ir::Type {
    let ptr_bits = env.pointer_type().bits();
    ir::Type::int((2 * ptr_bits).try_into().unwrap()).unwrap()
}

/// Turns a (possibly null) reference to a continuation object into a tuple
/// (revision, contref_ptr). If `contobj` denotes a wasm null reference, the
/// contref_ptr part will be a null pointer.
pub(crate) fn deconstruct<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    pos: &mut cranelift_codegen::cursor::FuncCursor,
    contobj: ir::Value,
) -> (ir::Value, ir::Value) {
    debug_assert_eq!(pos.func.dfg.value_type(contobj), fatpointer_type(env));
    let ptr_ty = env.pointer_type();
    let ptr_bits = ptr_ty.bits();

    let contref = pos.ins().ireduce(ptr_ty, contobj);
    let shifted = pos.ins().ushr_imm(contobj, i64::from(ptr_bits));
    let revision_counter = pos.ins().ireduce(ptr_ty, shifted);

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
    let ptr_ty = env.pointer_type();
    let ptr_bits = ptr_ty.bits();
    let fat_ptr_ty = fatpointer_type(env);

    debug_assert_eq!(pos.func.dfg.value_type(contref_addr), ptr_ty);
    debug_assert_eq!(pos.func.dfg.value_type(revision_counter), ptr_ty);

    let contref_addr = pos.ins().uextend(fat_ptr_ty, contref_addr);
    let revision_counter = pos.ins().uextend(fat_ptr_ty, revision_counter);
    let shifted_counter = pos.ins().ishl_imm(revision_counter, i64::from(ptr_bits));
    let contobj = pos.ins().bor(shifted_counter, contref_addr);

    contobj
}
