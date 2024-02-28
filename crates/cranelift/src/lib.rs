//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of the `wasmtime_environ::Compiler`
//! and `wasmtime_environ::CompilerBuilder` traits.

use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, WasmFuncType, WasmHeapType, WasmValType};
use target_lexicon::Architecture;
use wasmtime_cranelift_shared::CompiledFunctionMetadata;

pub use builder::builder;
use wasmtime_environ::Tunables;

mod builder;
mod compiler;
mod debug;
mod func_environ;
mod gc;

type CompiledFunctionsMetadata<'a> = PrimaryMap<DefinedFuncIndex, &'a CompiledFunctionMetadata>;

/// Trap code used for debug assertions we emit in our JIT code.
const DEBUG_ASSERT_TRAP_CODE: u16 = u16::MAX;

/// Creates a new cranelift `Signature` with no wasm params/results for the
/// given calling convention.
///
/// This will add the default vmctx/etc parameters to the signature returned.
fn blank_sig(isa: &dyn TargetIsa, call_conv: CallConv) -> ir::Signature {
    let pointer_type = isa.pointer_type();
    let mut sig = ir::Signature::new(call_conv);
    // Add the caller/callee `vmctx` parameters.
    sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));
    sig.params.push(ir::AbiParam::new(pointer_type));
    return sig;
}

/// TODO FITZGEN
fn unbarriered_store_type_at_offset(
    isa: &dyn TargetIsa,
    pos: &mut FuncCursor,
    ty: WasmValType,
    base: ir::Value,
    offset: i32,
    value: ir::Value,
) {
    let ir_ty = value_type(isa, ty);
    if ir_ty.is_ref() {
        let value = pos
            .ins()
            .bitcast(ir_ty.as_int(), ir::MemFlags::new(), value);
        let truncated = match isa.pointer_bytes() {
            4 => value,
            8 => pos.ins().ireduce(ir::types::I32, value),
            _ => unreachable!(),
        };
        pos.ins()
            .store(ir::MemFlags::trusted(), truncated, base, offset);
    } else {
        pos.ins()
            .store(ir::MemFlags::trusted(), value, base, offset);
    }
}

/// TODO FITZGEN
fn unbarriered_load_type_at_offset(
    isa: &dyn TargetIsa,
    pos: &mut FuncCursor,
    ty: WasmValType,
    base: ir::Value,
    offset: i32,
) -> ir::Value {
    let ir_ty = value_type(isa, ty);
    if ir_ty.is_ref() {
        let gc_ref = pos
            .ins()
            .load(ir::types::I32, ir::MemFlags::trusted(), base, offset);
        let extended = match isa.pointer_bytes() {
            4 => gc_ref,
            8 => pos.ins().uextend(ir::types::I64, gc_ref),
            _ => unreachable!(),
        };
        pos.ins().bitcast(ir_ty, ir::MemFlags::new(), extended)
    } else {
        pos.ins().load(ir_ty, ir::MemFlags::trusted(), base, offset)
    }
}

/// Returns the corresponding cranelift type for the provided wasm type.
fn value_type(isa: &dyn TargetIsa, ty: WasmValType) -> ir::types::Type {
    match ty {
        WasmValType::I32 => ir::types::I32,
        WasmValType::I64 => ir::types::I64,
        WasmValType::F32 => ir::types::F32,
        WasmValType::F64 => ir::types::F64,
        WasmValType::V128 => ir::types::I8X16,
        WasmValType::Ref(rt) => reference_type(rt.heap_type, isa.pointer_type()),
    }
}

/// Get the Cranelift signature with the native calling convention for the given
/// Wasm function type.
///
/// This parameters will start with the callee and caller VM contexts, followed
/// by the translation of each of the Wasm parameter types to native types. The
/// results are the Wasm result types translated to native types.
///
/// The signature uses the wasmtime variant of the target's default calling
/// convention. The only difference from the default calling convention is how
/// multiple results are handled.
///
/// When there is only a single result, or zero results, these signatures are
/// suitable for calling from the host via
///
/// ```ignore
/// unsafe extern "C" fn(
///     callee_vmctx: *mut VMOpaqueContext,
///     caller_vmctx: *mut VMOpaqueContext,
///     // ...wasm parameter types...
/// ) -> // ...wasm result type...
/// ```
///
/// When there are more than one results, these signatures are suitable for
/// calling from the host via
///
/// ```ignore
/// unsafe extern "C" fn(
///     callee_vmctx: *mut VMOpaqueContext,
///     caller_vmctx: *mut VMOpaqueContext,
///     // ...wasm parameter types...
///     retptr: *mut (),
/// ) -> // ...wasm result type 0...
/// ```
///
/// where the first result is returned directly and the rest via the return
/// pointer.
fn native_call_signature(isa: &dyn TargetIsa, wasm: &WasmFuncType) -> ir::Signature {
    let mut sig = blank_sig(isa, CallConv::triple_default(isa.triple()));
    let cvt = |ty: &WasmValType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm.params().iter().map(&cvt));
    if let Some(first_ret) = wasm.returns().get(0) {
        sig.returns.push(cvt(first_ret));
    }
    if wasm.returns().len() > 1 {
        sig.params.push(ir::AbiParam::new(isa.pointer_type()));
    }
    sig
}

/// Get the Cranelift signature for all array-call functions, that is:
///
/// ```ignore
/// unsafe extern "C" fn(
///     callee_vmctx: *mut VMOpaqueContext,
///     caller_vmctx: *mut VMOpaqueContext,
///     values_ptr: *mut ValRaw,
///     values_len: usize,
/// )
/// ```
///
/// This signature uses the target's default calling convention.
///
/// Note that regardless of the Wasm function type, the array-call calling
/// convention always uses that same signature.
fn array_call_signature(isa: &dyn TargetIsa) -> ir::Signature {
    let mut sig = blank_sig(isa, CallConv::triple_default(isa.triple()));
    // The array-call signature has an added parameter for the `values_vec`
    // input/output buffer in addition to the size of the buffer, in units
    // of `ValRaw`.
    sig.params.push(ir::AbiParam::new(isa.pointer_type()));
    sig.params.push(ir::AbiParam::new(isa.pointer_type()));
    sig
}

/// Get the internal Wasm calling convention signature for the given type.
fn wasm_call_signature(
    isa: &dyn TargetIsa,
    wasm_func_ty: &WasmFuncType,
    tunables: &Tunables,
) -> ir::Signature {
    // NB: this calling convention in the near future is expected to be
    // unconditionally switched to the "tail" calling convention once all
    // platforms have support for tail calls.
    //
    // Also note that the calling convention for wasm functions is purely an
    // internal implementation detail of cranelift and Wasmtime. Native Rust
    // code does not interact with raw wasm functions and instead always
    // operates through trampolines either using the `array_call_signature` or
    // `native_call_signature` where the default platform ABI is used.
    let call_conv = match isa.triple().architecture {
        // If the tail calls proposal is enabled, we must use the tail calling
        // convention. We don't use it by default yet because of
        // https://github.com/bytecodealliance/wasmtime/issues/6759
        arch if tunables.tail_callable => {
            assert_ne!(
                arch,
                Architecture::S390x,
                "https://github.com/bytecodealliance/wasmtime/issues/6530"
            );
            CallConv::Tail
        }

        // The winch calling convention is only implemented for x64 and aarch64
        arch if tunables.winch_callable => {
            assert!(
                matches!(arch, Architecture::X86_64),
                "The Winch calling convention is only implemented for x86_64"
            );
            CallConv::Winch
        }

        // On s390x the "wasmtime" calling convention is used to give vectors
        // little-endian lane order at the ABI layer which should reduce the
        // need for conversion when operating on vector function arguments. By
        // default vectors on s390x are otherwise in big-endian lane order which
        // would require conversions.
        Architecture::S390x => CallConv::WasmtimeSystemV,

        // All other platforms pick "fast" as the calling convention since it's
        // presumably, well, the fastest.
        _ => CallConv::Fast,
    };
    let mut sig = blank_sig(isa, call_conv);
    let cvt = |ty: &WasmValType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm_func_ty.params().iter().map(&cvt));
    sig.returns.extend(wasm_func_ty.returns().iter().map(&cvt));
    sig
}

/// Returns the reference type to use for the provided wasm type.
fn reference_type(wasm_ht: WasmHeapType, pointer_type: ir::Type) -> ir::Type {
    match wasm_ht {
        WasmHeapType::Func | WasmHeapType::Concrete(_) | WasmHeapType::NoFunc => pointer_type,
        WasmHeapType::Extern | WasmHeapType::Any | WasmHeapType::I31 | WasmHeapType::None => {
            match pointer_type {
                ir::types::I32 => ir::types::R32,
                ir::types::I64 => ir::types::R64,
                _ => panic!("unsupported pointer type"),
            }
        }
    }
}

/// If this bit is set on a GC reference, then the GC reference is actually an
/// unboxed `i31`.
///
/// Must be kept in sync with
/// `wasmtime_runtime::gc::VMGcRef::I31_REF_DISCRIMINANT`.
const I31_REF_DISCRIMINANT: u32 = 1;
