use cranelift_codegen::ir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::types::I64;

/// The Cranelfift type used to represent all of the following:
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

    let (lsbs, msbs) = pos.ins().isplit(contobj);

    let (revision_counter, contref) = match env.isa().endianness() {
        ir::Endianness::Little => (lsbs, msbs),
        ir::Endianness::Big => {
            let pad_bits = 64 - env.pointer_type().bits();
            let contref = pos.ins().ushr_imm(lsbs, pad_bits as i64);
            (msbs, contref)
        }
    };
    let contref = if env.pointer_type().bits() < I64.bits() {
        pos.ins().ireduce(env.pointer_type(), contref)
    } else {
        contref
    };
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
    let contref_addr = if env.pointer_type().bits() < I64.bits() {
        pos.ins().uextend(I64, contref_addr)
    } else {
        contref_addr
    };
    let (msbs, lsbs) = match env.isa().endianness() {
        ir::Endianness::Little => (contref_addr, revision_counter),
        ir::Endianness::Big => {
            let pad_bits = 64 - env.pointer_type().bits();
            let lsbs = pos.ins().ishl_imm(contref_addr, pad_bits as i64);
            (revision_counter, lsbs)
        }
    };

    let lsbs = pos.ins().uextend(ir::types::I128, lsbs);
    let msbs = pos.ins().uextend(ir::types::I128, msbs);
    let msbs = pos.ins().ishl_imm(msbs, 64);
    pos.ins().bor(lsbs, msbs)
}
