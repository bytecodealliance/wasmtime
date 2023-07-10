//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of the `wasmtime_environ::Compiler`
//! and `wasmtime_environ::CompilerBuilder` traits.

use cranelift_codegen::ir;
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, WasmFuncType, WasmType};
use target_lexicon::Architecture;
use wasmtime_cranelift_shared::CompiledFunctionMetadata;

pub use builder::builder;

mod builder;
mod compiler;
mod debug;
mod func_environ;

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

/// Returns the corresponding cranelift type for the provided wasm type.
fn value_type(isa: &dyn TargetIsa, ty: WasmType) -> ir::types::Type {
    match ty {
        WasmType::I32 => ir::types::I32,
        WasmType::I64 => ir::types::I64,
        WasmType::F32 => ir::types::F32,
        WasmType::F64 => ir::types::F64,
        WasmType::V128 => ir::types::I8X16,
        WasmType::Ref(rt) => reference_type(rt.heap_type, isa.pointer_type()),
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
    let cvt = |ty: &WasmType| ir::AbiParam::new(value_type(isa, *ty));
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
fn wasm_call_signature(isa: &dyn TargetIsa, wasm_func_ty: &WasmFuncType) -> ir::Signature {
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
    let cvt = |ty: &WasmType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm_func_ty.params().iter().map(&cvt));
    sig.returns.extend(wasm_func_ty.returns().iter().map(&cvt));
    sig
}

/// Returns the reference type to use for the provided wasm type.
fn reference_type(wasm_ht: cranelift_wasm::WasmHeapType, pointer_type: ir::Type) -> ir::Type {
    match wasm_ht {
        cranelift_wasm::WasmHeapType::Func | cranelift_wasm::WasmHeapType::TypedFunc(_) => {
            pointer_type
        }
        cranelift_wasm::WasmHeapType::Extern => match pointer_type {
            ir::types::I32 => ir::types::R32,
            ir::types::I64 => ir::types::R64,
            _ => panic!("unsupported pointer type"),
        },
    }
}
