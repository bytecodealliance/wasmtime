//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of the `wasmtime_environ::Compiler`
//! and `wasmtime_environ::CompilerBuilder` traits.

use cranelift_codegen::ir;
use cranelift_codegen::isa::{unwind::UnwindInfo, CallConv, TargetIsa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, WasmFuncType, WasmType};
use target_lexicon::{Architecture, CallingConvention};
use wasmtime_cranelift_shared::Relocation;
use wasmtime_environ::{FilePos, InstructionAddressMap, TrapInformation};

pub use builder::builder;

mod builder;
mod compiler;
mod debug;
mod func_environ;

type CompiledFunctions<'a> = PrimaryMap<DefinedFuncIndex, &'a CompiledFunction>;

/// Trap code used for debug assertions we emit in our JIT code.
const DEBUG_ASSERT_TRAP_CODE: u16 = u16::MAX;

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Default)]
pub struct CompiledFunction {
    /// The machine code for this function.
    body: Vec<u8>,

    /// The unwind information.
    unwind_info: Option<UnwindInfo>,

    /// Information used to translate from binary offsets back to the original
    /// location found in the wasm input.
    address_map: FunctionAddressMap,

    /// Metadata about traps in this module, mapping code offsets to the trap
    /// that they may cause.
    traps: Vec<TrapInformation>,

    relocations: Vec<Relocation>,
    value_labels_ranges: cranelift_codegen::ValueLabelsRanges,
    sized_stack_slots: ir::StackSlots,
    alignment: u32,
}

/// Function and its instructions addresses mappings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FunctionAddressMap {
    /// An array of data for the instructions in this function, indicating where
    /// each instruction maps back to in the original function.
    ///
    /// This array is sorted least-to-greatest by the `code_offset` field.
    /// Additionally the span of each `InstructionAddressMap` is implicitly the
    /// gap between it and the next item in the array.
    instructions: Box<[InstructionAddressMap]>,

    /// Function's initial offset in the source file, specified in bytes from
    /// the front of the file.
    start_srcloc: FilePos,

    /// Function's end offset in the source file, specified in bytes from
    /// the front of the file.
    end_srcloc: FilePos,

    /// Generated function body offset if applicable, otherwise 0.
    body_offset: usize,

    /// Generated function body length.
    body_len: u32,
}

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

/// Returns the default calling convention for the `isa` provided.
///
/// Note that this calling convention is used for exported functions.
fn wasmtime_call_conv(isa: &dyn TargetIsa) -> CallConv {
    match isa.triple().default_calling_convention() {
        Ok(CallingConvention::AppleAarch64) => CallConv::WasmtimeAppleAarch64,
        Ok(CallingConvention::SystemV) | Err(()) => CallConv::WasmtimeSystemV,
        Ok(CallingConvention::WindowsFastcall) => CallConv::WasmtimeFastcall,
        Ok(unimp) => unimplemented!("calling convention: {:?}", unimp),
    }
}

/// Appends the types of the `wasm` function signature into the `sig` signature
/// provided.
///
/// Typically the `sig` signature will have been created from [`blank_sig`]
/// above.
fn push_types(isa: &dyn TargetIsa, sig: &mut ir::Signature, wasm: &WasmFuncType) {
    let cvt = |ty: &WasmType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm.params().iter().map(&cvt));
    sig.returns.extend(wasm.returns().iter().map(&cvt));
}

/// Returns the corresponding cranelift type for the provided wasm type.
fn value_type(isa: &dyn TargetIsa, ty: WasmType) -> ir::types::Type {
    match ty {
        WasmType::I32 => ir::types::I32,
        WasmType::I64 => ir::types::I64,
        WasmType::F32 => ir::types::F32,
        WasmType::F64 => ir::types::F64,
        WasmType::V128 => ir::types::I8X16,
        WasmType::FuncRef | WasmType::ExternRef => reference_type(ty, isa.pointer_type()),
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
    let mut sig = blank_sig(isa, wasmtime_call_conv(isa));
    push_types(isa, &mut sig, wasm);
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
    let call_conv = if isa.triple().default_calling_convention().ok()
        == Some(CallingConvention::AppleAarch64)
    {
        // FIXME: We need an Apple-specific calling convention, so that
        // Cranelift's ABI implementation generates unwinding directives
        // about pointer authentication usage, so we can't just use
        // `CallConv::Fast`.
        CallConv::WasmtimeAppleAarch64
    } else if isa.triple().architecture == Architecture::S390x {
        // On S390x we need a Wasmtime calling convention to ensure
        // we're using little-endian vector lane order.
        wasmtime_call_conv(isa)
    } else {
        CallConv::Fast
    };

    let mut sig = blank_sig(isa, call_conv);
    push_types(isa, &mut sig, wasm_func_ty);
    sig
}

/// Returns the reference type to use for the provided wasm type.
fn reference_type(wasm_ty: cranelift_wasm::WasmType, pointer_type: ir::Type) -> ir::Type {
    match wasm_ty {
        cranelift_wasm::WasmType::FuncRef => pointer_type,
        cranelift_wasm::WasmType::ExternRef => match pointer_type {
            ir::types::I32 => ir::types::R32,
            ir::types::I64 => ir::types::R64,
            _ => panic!("unsupported pointer type"),
        },
        _ => panic!("unsupported Wasm reference type"),
    }
}
