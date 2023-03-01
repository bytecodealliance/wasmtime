//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of the `wasmtime_environ::Compiler`
//! and `wasmtime_environ::CompilerBuilder` traits.

use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::isa::{self, unwind::UnwindInfo, CallConv, TargetIsa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmFuncType, WasmType};
use target_lexicon::{Architecture, CallingConvention};
use wasmtime_environ::{
    FilePos, InstructionAddressMap, ModuleTranslation, ModuleTypes, TrapInformation,
};

mod compiler;
mod debug;
mod func_environ;
mod obj;

type CompiledFunctions<'a> = PrimaryMap<DefinedFuncIndex, &'a CompiledFunction>;

pub fn builder() -> Box<dyn wasmtime_environ::CompilerBuilder> {
    wasmtime_cranelift_shared::builder(
        |triple| isa::lookup(triple).map_err(|e| e.into()),
        |isa, opts| {
            Ok(Box::new(crate::compiler::Compiler::new(
                isa?,
                opts.cache_store.clone(),
                opts.linkopts.clone(),
            )))
        },
    )
}

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

/// A record of a relocation to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Relocation {
    /// The relocation code.
    reloc: binemit::Reloc,
    /// Relocation target.
    reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    addend: binemit::Addend,
}

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
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

/// Returns a cranelift signature suitable to indirectly call the wasm signature
/// specified by `wasm`.
///
/// This will implicitly use the default calling convention for `isa` since to
/// indirectly call a wasm function it must be possibly exported somehow (e.g.
/// this assumes the function target to call doesn't use the "fast" calling
/// convention).
fn indirect_signature(isa: &dyn TargetIsa, wasm: &WasmFuncType) -> ir::Signature {
    let mut sig = blank_sig(isa, wasmtime_call_conv(isa));
    push_types(isa, &mut sig, wasm);
    return sig;
}

/// Returns the cranelift fucntion signature of the function specified.
///
/// Note that this will determine the calling convention for the function, and
/// namely includes an optimization where functions never exported from a module
/// use a custom theoretically faster calling convention instead of the default.
fn func_signature(
    isa: &dyn TargetIsa,
    translation: &ModuleTranslation,
    types: &ModuleTypes,
    index: FuncIndex,
) -> ir::Signature {
    let func = &translation.module.functions[index];
    let call_conv = match translation.module.defined_func_index(index) {
        // If this is a defined function in the module and it doesn't escape
        // then we can optimize this function to use the fastest calling
        // convention since it's purely an internal implementation detail of
        // the module itself.
        Some(_idx) if !func.is_escaping() => {
            let on_apple_aarch64 = isa
                .triple()
                .default_calling_convention()
                .unwrap_or(CallingConvention::SystemV)
                == CallingConvention::AppleAarch64;

            if on_apple_aarch64 {
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
            }
        }

        // ... otherwise if it's an imported function or if it's a possibly
        // exported function then we use the default ABI wasmtime would
        // otherwise select.
        _ => wasmtime_call_conv(isa),
    };
    let mut sig = blank_sig(isa, call_conv);
    push_types(isa, &mut sig, &types[func.signature]);
    return sig;
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
